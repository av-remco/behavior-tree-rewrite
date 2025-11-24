#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use actify::Handle;
    use log::warn;
    use tokio::sync::mpsc::Receiver;
    use std::collections::HashMap;
    use tokio::time::{Duration, sleep};
    use crate::bt::Ready;
    use crate::conversion::converter::convert_bt;
    use crate::execution::traversal::{search_next, search_start};
    use crate::logging::load_logger;
    use crate::nodes::action::mocking::MockAction;
    use crate::nodes_bin::node_handle::NodeHandle;
    use crate::nodes_bin::node_status::Status;
    use crate::{BT, Condition, Failure, Fallback, Sequence, Success, Wait};
    use logtest::Logger;

    // * Tests for search_down()
    #[tokio::test]
    async fn test_auto_failure() {
        let action1 = Failure::new();
        let bt = BT::new().root(action1.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            action1,                  // action visited
        ]);
    }

    #[tokio::test]
    async fn test_auto_success() {
        let action1 = Success::new();
        let bt = BT::new().root(action1.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            action1,
        ]);
    }

    #[tokio::test]
    async fn test_condition_true_stops_at_condition() {
        let cond = Condition::new("cond1", Handle::new(5), |x| x > 0);
        let bt = BT::new().root(cond.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            cond,                    // condition entered → stops
        ]);
    }

    #[tokio::test]
    async fn test_sequence_hits_first_action() {
        let a1 = Success::new();
        let a2 = Success::new();
        let seq = Sequence::new(vec![a1.clone(), a2.clone()]);

        let bt = BT::new().root(seq.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            seq,                     // enter sequence
            a1,                      // first actionable child
        ]);
    }

    #[tokio::test]
    async fn test_sequence_condition_stops_sequence() {
        let cond = Condition::new("cond_seq", Handle::new(0), |x| x > 0);
        let a2 = Success::new();

        let seq = Sequence::new(vec![cond.clone(), a2.clone()]);
        let bt = BT::new().root(seq.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            seq,                     // enter sequence
            cond,                    // condition → STOP
        ]);
    }

    #[tokio::test]
    async fn test_fallback_hits_first_action() {
        let fail1 = Failure::new();
        let succ = Success::new();

        let fb = Fallback::new(vec![fail1.clone(), succ.clone()]);
        let bt = BT::new().root(fb.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            fb,                      // enter fallback
            fail1,                   // first child
        ]);
    }

    #[tokio::test]
    async fn test_fallback_condition_as_first_child() {
        let cond = Condition::new("cond_fb", Handle::new(0), |x| x > 0);
        let a2  = Success::new();

        let fb = Fallback::new(vec![cond.clone(), a2.clone()]);
        let bt = BT::new().root(fb.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            fb,                      // enter fallback
            cond,                    // stops → fallback does NOT continue
        ]);
    }

    #[tokio::test]
    async fn test_nested_sequence_and_fallback() {
        // Sequence:
        //   cond  → stops (no fallback entered)
        //   fallback(fail, act)
        //
        // Condition prevents entering fallback.

        let cond = Condition::new("cond_nested", Handle::new(1), |x| x > 0);

        let fail = Failure::new();
        let act = Success::new();
        let fb = Fallback::new(vec![fail.clone(), act.clone()]);

        let seq = Sequence::new(vec![cond.clone(), fb.clone()]);
        let bt = BT::new().root(seq.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            seq,                     // enter sequence
            cond,                    // stops → fallback never visited
        ]);
    }

    #[tokio::test]
    async fn test_fallback_sequence_condition_then_action() {
        // fallback:
        //   sequence(cond → action)
        //   action2
        //
        // Sequence will stop at condition → so fallback won't go deeper.

        let cond = Condition::new("cond1", Handle::new(3), |x| x > 0);

        let a1 = Success::new();
        let seq = Sequence::new(vec![cond.clone(), a1.clone()]);

        let a2 = Success::new();
        let fb = Fallback::new(vec![seq.clone(), a2.clone()]);

        let bt = BT::new().root(fb.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            fb,                      // enter fallback
            seq,                     // enter sequence
            cond,                    // stops sequence
        ]);
    }


    // * Tests for search_next(&bt, )
    #[tokio::test]
    async fn test_search_next_sequence_continue() {
        // sequence: cond  → a1 → a2
        // Condition succeeds, so sequence continues to first action.

        let cond = Condition::new("cond1", Handle::new(1), |x| x > 0);
        let a1 = Success::new();
        let a2 = Success::new();

        let seq = Sequence::new(vec![cond.clone(), a1.clone(), a2.clone()]);
        let bt = BT::new().root(seq.clone()).name("test_tree");

        // First search: stops at condition
        let start = search_start(&bt);
        assert_eq!(start, vec![seq.clone(), cond.clone()]);

        // Now simulate the condition result
        let next = search_next(&bt, start.clone(), &Status::Success);

        assert_eq!(next, vec![
            seq.clone(),
            a1.clone(),   // first action after successful condition
        ]);
    }

    #[tokio::test]
    async fn test_search_next_sequence_stops_on_failure() {
        // sequence: cond → a1
        // Condition fails => sequence halts, next search should NOT go to a1.

        let cond = Condition::new("cond1", Handle::new(10), |x| x == 0);
        let a1 = Success::new();

        let seq = Sequence::new(vec![cond.clone(), a1.clone()]);
        let bt = BT::new().root(seq.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![seq.clone(), cond.clone()]);

        // Condition failed
        let next = search_next(&bt, start.clone(), &Status::Failure);

        // Sequence terminates → no deeper path
        assert_eq!(next, vec![]);
    }

    #[tokio::test]
    async fn test_search_next_fallback_continue_on_failure() {
        // fallback:
        //   cond (fails)
        //   a1 (fallback continues)

        let cond = Condition::new("c1", Handle::new(3), |x| x == 0);
        let a1 = Success::new();

        let fb = Fallback::new(vec![cond.clone(), a1.clone()]);
        let bt = BT::new().root(fb.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![fb.clone(), cond.clone()]);

        // cond fails => fallback continues to next child
        let next = search_next(&bt, start.clone(), &Status::Failure);

        assert_eq!(next, vec![
            fb.clone(),
            a1.clone(),
        ]);
    }

    #[tokio::test]
    async fn test_search_next_fallback_stop_on_success() {
        // fallback:
        //   cond → a1
        //
        // Condition succeeds → fallback does NOT continue to a1.

        let cond = Condition::new("c1", Handle::new(2), |x| x > 0);
        let a1 = Success::new();

        let fb = Fallback::new(vec![cond.clone(), a1.clone()]);
        let bt = BT::new().root(fb.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![fb.clone(), cond.clone()]);

        // cond success => fallback stops
        let next = search_next(&bt, start.clone(), &Status::Success);

        assert_eq!(next, vec![]);
    }

    #[tokio::test]
    async fn test_search_next_nested() {
        // fb:
        //   seq(cond → a1)
        //   a2
        //
        // search_start hits condition inside sequence
        // next after FAILURE should go to fallback's second child a2

        let cond = Condition::new("c1", Handle::new(5), |x| x < 0);
        let a1 = Success::new();
        let seq = Sequence::new(vec![cond.clone(), a1.clone()]);

        let a2 = Success::new();
        let fb = Fallback::new(vec![seq.clone(), a2.clone()]);

        let bt = BT::new().root(fb.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![
            fb.clone(),
            seq.clone(),
            cond.clone()
        ]);

        // Condition fails
        let next = search_next(&bt, start.clone(), &Status::Failure);

        // fallback tries second child (a2)
        assert_eq!(next, vec![
            fb.clone(),
            a2.clone(),
        ]);
    }

    #[tokio::test]
    async fn test_search_next_on_root_returns_empty() {
        // Root → child action
        // search_start(&bt) stops at the first actionable node (the child)
        //
        // But search_next applied on a trace that only contains the root
        // should return vec![], since there is no higher-level parent.

        let a1 = Success::new();

        let bt = BT::new().root(a1.clone()).name("test_tree");

        // Try search_next after the root returns any Status (Success or Failure)
        let fst_trace = search_start(&bt);
        let snd_trace = search_next(&bt, fst_trace.clone(), &Status::Success);
        let trd_trace = search_next(&bt, fst_trace.clone(), &Status::Failure);

        assert_eq!(fst_trace, vec![a1.clone()]);
        assert_eq!(snd_trace, Vec::<NodeHandle>::new());
        assert_eq!(trd_trace, Vec::<NodeHandle>::new());
    }
}