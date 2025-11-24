#[cfg(test)]
#[allow(unused_imports)]
mod tests {

    // * Tests for bt.execute()

    use std::time::Duration;

    use actify::Handle;
    use tokio::time::sleep;

    use crate::{BT, Condition, Failure, Fallback, Sequence, Success, Wait, bt::Processing, logging::load_logger, nodes::action::mocking::MockAction, nodes_bin::node_status::Status};

    #[tokio::test]
    async fn test_execute_simple_success() {
        let action = Success::new();
        let bt = BT::new(action.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_simple_failure() {
        let action = Failure::new();
        let bt = BT::new(action.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_condition_true() {
        let cond = Condition::new("cond_true", Handle::new(10), |x| x > 0);
        let bt = BT::new(cond.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_condition_false() {
        let cond = Condition::new("cond_false", Handle::new(0), |x| x > 5);
        let bt = BT::new(cond.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_sequence_all_success() {
        let a1 = Success::new();
        let a2 = Success::new();
        let seq = Sequence::new(vec![a1.clone(), a2.clone()]);

        let bt = BT::new(seq.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_sequence_stops_on_failure() {
        let a1 = Success::new();
        let a2 = Failure::new();
        let seq = Sequence::new(vec![a1.clone(), a2.clone()]);

        let bt = BT::new(seq.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_fallback_first_success() {
        let s1 = Success::new();
        let f1 = Failure::new();
        let fb = Fallback::new(vec![s1.clone(), f1.clone()]);

        let bt = BT::new(fb.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_fallback_second_success() {
        let f1 = Failure::new();
        let s1 = Success::new();
        let fb = Fallback::new(vec![f1.clone(), s1.clone()]);

        let bt = BT::new(fb.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_fallback_all_fail() {
        let f1 = Failure::new();
        let f2 = Failure::new();
        let fb = Fallback::new(vec![f1.clone(), f2.clone()]);

        let bt = BT::new(fb.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_nested_sequence_fallback() {
        // seq:
        //   cond (false)
        //   fallback(fail, success)
        //
        // Because condition stops immediately → seq returns FAILURE

        let cond = Condition::new("nested", Handle::new(0), |x| x > 0);
        let fb = Fallback::new(vec![Failure::new(), Success::new()]);
        let seq = Sequence::new(vec![cond.clone(), fb.clone()]);

        let bt = BT::new(seq.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;

        // sequence stops at cond → cond returns false → seq returns false
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_wait_action() {
        let wait = Wait::new(Duration::from_millis(50));
        let bt = BT::new(wait.clone(), "exec_tree");
        let bt = bt.build();

        let result = bt.execute().await;
        assert_eq!(result.result(), true);
    }

    //  Cond1
    //    |
    // Action1
    #[tokio::test]
    async fn test_condition_interrupt() {
        let handle = Handle::new(1);

        let action1 = MockAction::new(1);
        let cond1 = Condition::new("cond", handle.clone(), |x| x > 0);
        let seq = Sequence::new(vec![cond1, action1]);
        let bt = BT::new(seq,"");

        let (res, _) = tokio::join!(
            bt.build().execute(),
            async {
                sleep(Duration::from_millis(200)).await;
                handle.set(-1).await;
            }
        );

        assert_eq!(res.result(), false);
    }

    #[tokio::test]
    async fn test_error_propagation_in_sequence() {
        // Sequence
        //   |
        // Action1 -> ActionErr -> Action2
        let action1 = MockAction::new(1);
        let action_err = MockAction::new_error(2);
        let action2 = MockAction::new(3);
        let seq = Sequence::new(vec![action1, action_err, action2]);
        let bt = BT::new(seq, "");

        let res = bt.build().execute().await;
        assert_eq!(res.result(), false);
    }

    #[tokio::test]
    async fn test_two_conditions_switching() {
        // Cond1 -> Cond2 -> Action1
        let handle1 = Handle::new(1);
        let handle2 = Handle::new(1);
        
        let cond1 = Condition::new("cond1", handle1.clone(), |x| x > 0);
        let cond2 = Condition::new("cond2", handle2.clone(), |x| x > 0);
        let action1 = MockAction::new(1);
        
        let seq = Sequence::new(vec![cond1, cond2, action1]);
        let bt = BT::new(seq, "");

        let (res, _, _) = tokio::join!(
            bt.build().execute(),
            async {
                sleep(Duration::from_millis(200)).await;
                handle2.set(0).await; // make cond2 fail mid-execution
            },
            async {
                sleep(Duration::from_millis(200)).await;
                handle1.set(0).await; // make cond1 fail mid-execution
            }
        );

        assert_eq!(res.result(), false);
    }

    #[tokio::test]
    async fn test_condition_fails_mid_sequence() {
        load_logger();
        // Cond1 -> Cond2 -> Action1
        let handle1 = Handle::new(1);
        let handle2 = Handle::new(1);
        
        let cond1 = Condition::new("cond1", handle1.clone(), |x| x > 0);
        let cond2 = Condition::new("cond2", handle2.clone(), |x| x > 0);
        let action1 = MockAction::new(1);
        
        let seq = Sequence::new(vec![cond1, cond2, action1]);
        let bt = BT::new(seq, "");

        let (res, _) = tokio::join!(
            bt.build().execute(),
            async {
                sleep(Duration::from_millis(300)).await;
                handle2.set(0).await; // cond2 now fails mid-sequence
            }
        );

        assert_eq!(res.result(), false);
    }

    #[tokio::test]
    async fn test_multiple_conditions_toggle() {
        // Cond1 -> Cond2 -> Cond3 -> Action1
        let h1 = Handle::new(0);
        let h2 = Handle::new(0);
        let h3 = Handle::new(0);

        let cond1 = Condition::new("c1", h1.clone(), |x| x > 0);
        let cond2 = Condition::new("c2", h2.clone(), |x| x > 0);
        let cond3 = Condition::new("c3", h3.clone(), |x| x > 0);
        let action1 = MockAction::new(1);

        let seq = Sequence::new(vec![cond1, cond2, cond3, action1]);
        let bt = BT::new(seq, "");

        let (res, _) = tokio::join!(
            bt.build().execute(),
            async {
                sleep(Duration::from_millis(100)).await;
                h1.set(1).await; // first condition passes
                sleep(Duration::from_millis(100)).await;
                h2.set(1).await; // second condition passes
                sleep(Duration::from_millis(100)).await;
                h3.set(1).await; // third condition passes
            }
        );

        assert_eq!(res.result(), true);
    }

    #[tokio::test]
    async fn test_conditions_toggle_fail_then_recover() {
        // Cond1 -> Cond2 -> Action1
        let h1 = Handle::new(1);
        let h2 = Handle::new(1);

        let cond1 = Condition::new("c1", h1.clone(), |x| x > 0);
        let cond2 = Condition::new("c2", h2.clone(), |x| x > 0);
        let action1 = MockAction::new(1);

        let seq = Sequence::new(vec![cond1, cond2, action1]);
        let bt = BT::new(seq, "");

        let (res, _) = tokio::join!(
            bt.build().execute(),
            async {
                sleep(Duration::from_millis(150)).await;
                h2.set(0).await; // cond2 fails mid-execution
                sleep(Duration::from_millis(150)).await;
                h2.set(1).await; // cond2 recovers
            }
        );

        assert_eq!(res.result(), true, "Tree should eventually succeed after condition recovers");
    }

}