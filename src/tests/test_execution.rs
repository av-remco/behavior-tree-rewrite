#[cfg(test)]
#[allow(unused_imports)]
mod tests {

    // * Tests for bt.execute()

    use std::time::Duration;

    use actify::Handle;

    use crate::{BT, Condition, Failure, Fallback, Sequence, Success, Wait, bt::Processing, nodes::action::mocking::MockAction, nodes_bin::node_status::Status};

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
}