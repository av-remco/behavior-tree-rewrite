#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use actify::Handle;
    use log::warn;
    use tokio::sync::mpsc::Receiver;
    use std::collections::HashMap;
    use tokio::time::{Duration, sleep};
    use crate::bt::Processing;
    use crate::conversion::converter::convert_bt;
    use crate::execution::traversal::{search_next, search_start};
    use crate::logging::load_logger;
    use crate::nodes::action::mocking::MockAction;
    use crate::nodes_bin::node_handle::NodeHandle;
    use crate::nodes_bin::node_status::Status;
    use crate::{BT, Condition, Failure, Fallback, Sequence, Success, Wait};
    use logtest::Logger;

    #[tokio::test]
    async fn test_convert_simple_action_root() {
        let action = MockAction::new(1);
        let bt = BT::new(action.clone(), "test_tree");
        let mut bt: BT<Processing> = bt.test_into_state();

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

        let bt = BT::new(seq, "test_tree");
        let mut bt: BT<Processing> = bt.test_into_state();

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

        let bt = BT::new(fb, "test_tree");
        let mut bt: BT<Processing> = bt.test_into_state();

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

        let bt = BT::new(seq, "test_tree");
        let mut bt: BT<Processing> = bt.test_into_state();

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

        let bt = BT::new(fb, "test_tree");
        let mut bt: BT<Processing> = bt.test_into_state();

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