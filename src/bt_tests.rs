#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use actify::Handle;
    use log::warn;
    use tokio::sync::mpsc::Receiver;
    use crate::bt::converter::convert_bt;
    use crate::bt::handle::Status;
    use crate::bt::traversal::{search_next, search_start};
    use crate::{BehaviorTree, NodeError, NodeHandle};
    use std::collections::HashMap;
    use tokio::time::{Duration, sleep};

    use crate::bt::*;
    use crate::bt::{
        action::{Failure, Success},
        condition::{Condition},
        selector::{Sequence, Fallback},
    };
    use crate::bt::action::mocking::MockAction;
    use crate::logging::load_logger;
    use crate::Wait;
    use logtest::Logger;

    // * Tests for search_down()
    #[tokio::test]
    async fn test_auto_failure() {
        let action1 = Failure::new();
        let bt = BehaviorTree::new_test(action1.clone());

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            action1,                  // action visited
        ]);
    }

    #[tokio::test]
    async fn test_auto_success() {
        let action1 = Success::new();
        let bt = BehaviorTree::new_test(action1.clone());

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            action1,
        ]);
    }

    #[tokio::test]
    async fn test_condition_true_stops_at_condition() {
        let cond = Condition::new("cond1", Handle::new(5), |x| x > 0);
        let bt = BehaviorTree::new_test(cond.clone());

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

        let bt = BehaviorTree::new_test(seq.clone());

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
        let bt = BehaviorTree::new_test(seq.clone());

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
        let bt = BehaviorTree::new_test(fb.clone());

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
        let bt = BehaviorTree::new_test(fb.clone());

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
        let bt = BehaviorTree::new_test(seq.clone());

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

        let bt = BehaviorTree::new_test(fb.clone());

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
        let bt = BehaviorTree::new_test(seq.clone());

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
        let bt = BehaviorTree::new_test(seq.clone());

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
        let bt = BehaviorTree::new_test(fb.clone());

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
        let bt = BehaviorTree::new_test(fb.clone());

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

        let bt = BehaviorTree::new_test(fb.clone());

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

        let bt = BehaviorTree::new_test(a1.clone());

        // Try search_next after the root returns any Status (Success or Failure)
        let fst_trace = search_start(&bt);
        let snd_trace = search_next(&bt, fst_trace.clone(), &Status::Success);
        let trd_trace = search_next(&bt, fst_trace.clone(), &Status::Failure);

        assert_eq!(fst_trace, vec![a1.clone()]);
        assert_eq!(snd_trace, Vec::<NodeHandle>::new());
        assert_eq!(trd_trace, Vec::<NodeHandle>::new());
    }

    // * Tests for convert_bt()

    #[tokio::test]
    async fn test_convert_simple_action_root() {
        let action = MockAction::new(1);
        let mut bt = BehaviorTree::new_test(action.clone());

        let map = convert_bt(&mut bt);

        // Only one node in the map
        assert_eq!(map.len(), 2);

        assert_eq!(
            map.get(&(action.clone(), Status::Success)),
            Some(&None)
        );

        assert_eq!(
            map.get(&(action.clone(), Status::Failure)),
            Some(&None)
        );
    }

    #[tokio::test]
    async fn test_condition_then_action() {
        let cond = Condition::new("cond1", Handle::new(1), |x| x > 0);
        let action = MockAction::new(1);

        let seq = Sequence::new(vec![cond.clone(), action.clone()]);

        let mut bt = BehaviorTree::new_test(seq);

        let map = convert_bt(&mut bt);
        println!("{:?}", map);

        // CONDITION → SUCCESS → ACTION
        assert_eq!(
            map.get(&(cond.clone(), Status::Success)),
            Some(&Some(action.clone()))
        );

        // CONDITION → FAILURE → root
        assert_eq!(
            map.get(&(cond.clone(), Status::Failure)),
            Some(&None)
        );

        // ACTION always ends → None
        assert_eq!(
            map.get(&(action.clone(), Status::Success)),
            Some(&None)
        );

        assert_eq!(
            map.get(&(action.clone(), Status::Failure)),
            Some(&None)
        );
    }

    // Fallback
    // ├─ cond → action1
    // └─ action2
    #[tokio::test]
    async fn test_fallback_cond_then_action_and_action2() {
        let cond = Condition::new("cond1", Handle::new(1), |x| x > 0);
        let a1 = MockAction::new(1);
        let a2 = MockAction::new(2);

        let fb = Fallback::new(vec![
            Sequence::new(vec![cond.clone(), a1.clone()]),
            a2.clone()
        ]);

        let mut bt = BehaviorTree::new_test(fb);

        let map = convert_bt(&mut bt);

        // Fallback logic:
        // Cond → Success → A1
        assert_eq!(
            map.get(&(cond.clone(), Status::Success)),
            Some(&Some(a1.clone()))
        );

        // Cond → Failure → A2
        assert_eq!(
            map.get(&(cond.clone(), Status::Failure)),
            Some(&Some(a2.clone()))
        );

        // A1 and A2 end the tree
        assert_eq!(map[&(a1.clone(), Status::Success)], None);
        assert_eq!(map[&(a1.clone(), Status::Failure)], Some(a2.clone()));

        assert_eq!(map[&(a2.clone(), Status::Success)], None);
        assert_eq!(map[&(a2.clone(), Status::Failure)], None);
    }

    // cond1 → cond2 → action
    #[tokio::test]
    async fn test_long_sequence_chain() {
        let cond1 = Condition::new("c1", Handle::new(1), |x| x > 0);
        let cond2 = Condition::new("c2", Handle::new(2), |x| x > 5);
        let act = MockAction::new(1);

        let seq = Sequence::new(vec![cond1.clone(), cond2.clone(), act.clone()]);

        let mut bt = BehaviorTree::new_test(seq);

        let map = convert_bt(&mut bt);

        // cond1 SUCCESS → cond2
        assert_eq!(
            map.get(&(cond1.clone(), Status::Success)),
            Some(&Some(cond2.clone()))
        );

        // cond2 SUCCESS → action
        assert_eq!(
            map.get(&(cond2.clone(), Status::Success)),
            Some(&Some(act.clone()))
        );

        // action SUCCESS → None
        assert_eq!(map[&(act.clone(), Status::Success)], None);

        // Any failure → root
        assert_eq!(map[&(cond1.clone(), Status::Failure)], None);
        assert_eq!(map[&(cond2.clone(), Status::Failure)], None);
        assert_eq!(map[&(act.clone(), Status::Failure)], None);
    }

    // Fallback
    // ├─ Sequence(cond1 → action1)
    // ├─ Sequence(cond2 → action2)
    // └─ action3

    #[tokio::test]
    async fn test_multiple_paths_and_selectors() {
        let cond1 = Condition::new("c1", Handle::new(1), |x| x > 0);
        let a1    = MockAction::new(1);

        let cond2 = Condition::new("c2", Handle::new(2), |x| x > 10);
        let a2    = MockAction::new(2);

        let a3 = MockAction::new(3);

        let fb = Fallback::new(vec![
            Sequence::new(vec![cond1.clone(), a1.clone()]),
            Sequence::new(vec![cond2.clone(), a2.clone()]),
            a3.clone(),
        ]);

        let mut bt = BehaviorTree::new_test(fb);

        let map = convert_bt(&mut bt);

        // cond1 SUCCESS → a1
        assert_eq!(map[&(cond1.clone(), Status::Success)], Some(a1.clone()));

        // cond1 FAILURE → cond2
        assert_eq!(map[&(cond1.clone(), Status::Failure)], Some(cond2.clone()));
        
        // a1 FAILURE → cond2
        assert_eq!(map[&(a1.clone(), Status::Failure)], Some(cond2.clone()));

        // cond2 SUCCESS → a2
        assert_eq!(map[&(cond2.clone(), Status::Success)], Some(a2.clone()));

        // cond2 FAILURE → a3
        assert_eq!(map[&(cond2.clone(), Status::Failure)], Some(a3.clone()));

        // a2 FAILURE → a3
        assert_eq!(map[&(a2.clone(), Status::Failure)], Some(a3.clone()));

        // a3 FAILURE -> None
        assert_eq!(map[&(a3.clone(), Status::Failure)], None);

        // All actions end cycle
        for a in [&a1, &a2, &a3] {
            assert_eq!(map[&(a.clone(), Status::Success)], None);
        }
    }


}