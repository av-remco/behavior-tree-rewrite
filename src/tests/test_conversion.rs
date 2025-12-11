#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use actify::Handle;
    use log::warn;
    use tokio::sync::mpsc::Receiver;
    use std::collections::HashMap;
    use tokio::time::{Duration, sleep};
    use crate::bt::Ready;
    use crate::execution::static_engine::converter::convert_bt;
    use crate::execution::traversal::{search_next, search_start};
    use crate::nodes::action::mocking::MockAction;
    use crate::nodes_bin::node::Node;
    use crate::nodes_bin::process_handle::ProcessHandle;
    use crate::nodes_bin::node_status::Status;
    use crate::{BT, Condition, Failure, Success, Wait};
    use logtest::Logger;

    #[tokio::test]
    async fn test_convert_simple_action_root() {
        let mut map = HashMap::new();
        let action = Success::new();
        let id = "a1".to_string();
        map.insert(id.clone(), action);
        let root = Node::Action(id);

        let bt = BT::new()
            .test_insert_map(map)
            .test_root(root.clone())
            .name("test_tree");

        let mut bt: BT<Ready> = bt.test_into_state();
        let map = convert_bt(&mut bt);

        assert_eq!(map.len(), 2);

        assert_eq!(map.get(&(root.clone(), Status::Success)), Some(&None));
        assert_eq!(map.get(&(root, Status::Failure)), Some(&None));
    }

    #[tokio::test]
    async fn test_condition_then_action() {
        let mut map = HashMap::new();
        let action = Success::new();
        let cond = Condition::new("cond1", Handle::new(1), |x| x > 0);
        let id1 = "c1".to_string();
        let id2 = "a1".to_string();
        map.insert(id1.clone(), action);
        map.insert(id2.clone(), cond);

        let root = Node::Sequence(vec![
            Node::Condition(id1.clone()),
            Node::Action(id2.clone()),
        ]);

        let bt = BT::new()
            .test_insert_map(map)
            .test_root(root)
            .name("test_tree");

        let mut bt: BT<Ready> = bt.test_into_state();
        let map = convert_bt(&mut bt);

        // cond SUCCESS → action
        assert_eq!(
            map.get(&(Node::Condition(id1.clone()), Status::Success)),
            Some(&Some(Node::Action(id2.clone())))
        );

        // cond FAILURE → None (end)
        assert_eq!(
            map.get(&(Node::Condition(id1.clone()), Status::Failure)),
            Some(&None)
        );

        // action → end
        assert_eq!(map[&(Node::Action(id2.clone()), Status::Success)], None);
        assert_eq!(map[&(Node::Action(id2.clone()), Status::Failure)], None);
    }


    // Fallback
    // ├─ cond → action1
    // └─ action2
    #[tokio::test]
    async fn test_fallback_cond_then_action_and_action2() {
        let mut map = HashMap::new();
        let cond = Condition::new("cond1", Handle::new(1), |x| x > 0);
        let a1 = MockAction::new(1);
        let a2 = MockAction::new(2);
        let id1 = "c1".to_string();
        let id2 = "a1".to_string();
        let id3 = "a2".to_string();
        map.insert(id1.clone(), cond);
        map.insert(id2.clone(), a1);
        map.insert(id3.clone(), a2);
        let seq = Node::Sequence(vec![
            Node::Condition(id1.clone()),
            Node::Action(id2.clone()),
        ]);
        let root = Node::Fallback(vec![
            seq,
            Node::Action(id3.clone()),
        ]);
        let bt = BT::new()
            .test_insert_map(map)
            .test_root(root)
            .name("test_tree");
        let mut bt: BT<Ready> = bt.test_into_state();
        let map = convert_bt(&mut bt);
        // Fallback logic:
        // Cond SUCCESS → A1
        assert_eq!(
            map.get(&(Node::Condition(id1.clone()), Status::Success)),
            Some(&Some(Node::Action(id2.clone())))
        );
        // Cond FAILURE → A2
        assert_eq!(
            map.get(&(Node::Condition(id1.clone()), Status::Failure)),
            Some(&Some(Node::Action(id3.clone())))
        );
        // A1 SUCCESS → end
        assert_eq!(map[&(Node::Action(id2.clone()), Status::Success)], None);
        // A1 FAILURE → A2
        assert_eq!(
            map.get(&(Node::Action(id2.clone()), Status::Failure)),
            Some(&Some(Node::Action(id3.clone())))
        );
        // A2 SUCCESS → end
        assert_eq!(map[&(Node::Action(id3.clone()), Status::Success)], None);
        // A2 FAILURE → end
        assert_eq!(map[&(Node::Action(id3.clone()), Status::Failure)], None);
    }


    // cond1 → cond2 → action
    #[tokio::test]
    async fn test_long_sequence_chain() {
        let mut map = HashMap::new();
        let cond1 = Condition::new("c1", Handle::new(1), |x| x > 0);
        let cond2 = Condition::new("c2", Handle::new(2), |x| x > 5);
        let act = MockAction::new(1);
        let id1 = "c1".to_string();
        let id2 = "c2".to_string();
        let id3 = "a1".to_string();
        map.insert(id1.clone(), cond1);
        map.insert(id2.clone(), cond2);
        map.insert(id3.clone(), act);
        let root = Node::Sequence(vec![
            Node::Condition(id1.clone()),
            Node::Condition(id2.clone()),
            Node::Action(id3.clone()),
        ]);
        let bt = BT::new()
            .test_insert_map(map)
            .test_root(root)
            .name("test_tree");
        let mut bt: BT<Ready> = bt.test_into_state();
        let map = convert_bt(&mut bt);
        // cond1 SUCCESS → cond2
        assert_eq!(
            map.get(&(Node::Condition(id1.clone()), Status::Success)),
            Some(&Some(Node::Condition(id2.clone())))
        );
        // cond2 SUCCESS → action
        assert_eq!(
            map.get(&(Node::Condition(id2.clone()), Status::Success)),
            Some(&Some(Node::Action(id3.clone())))
        );
        // action SUCCESS → None
        assert_eq!(map[&(Node::Action(id3.clone()), Status::Success)], None);
        // Any failure → root
        assert_eq!(map[&(Node::Condition(id1.clone()), Status::Failure)], None);
        assert_eq!(map[&(Node::Condition(id2.clone()), Status::Failure)], None);
        assert_eq!(map[&(Node::Action(id3.clone()), Status::Failure)], None);
    }


    // Fallback
    // ├─ Sequence(cond1 → action1)
    // ├─ Sequence(cond2 → action2)
    // └─ action3

    #[tokio::test]
    async fn test_multiple_paths_and_selectors() {
        let mut map = HashMap::new();
        let cond1 = Condition::new("c1", Handle::new(1), |x| x > 0);
        let a1 = MockAction::new(1);
        let cond2 = Condition::new("c2", Handle::new(2), |x| x > 10);
        let a2 = MockAction::new(2);
        let a3 = MockAction::new(3);
        let id1 = "c1".to_string();
        let id2 = "a1".to_string();
        let id3 = "c2".to_string();
        let id4 = "a2".to_string();
        let id5 = "a3".to_string();
        map.insert(id1.clone(), cond1);
        map.insert(id2.clone(), a1);
        map.insert(id3.clone(), cond2);
        map.insert(id4.clone(), a2);
        map.insert(id5.clone(), a3);
        let root = Node::Fallback(vec![
            Node::Sequence(vec![
                Node::Condition(id1.clone()),
                Node::Action(id2.clone()),
            ]),
            Node::Sequence(vec![
                Node::Condition(id3.clone()),
                Node::Action(id4.clone()),
            ]),
            Node::Action(id5.clone()),
        ]);
        let bt = BT::new()
            .test_insert_map(map)
            .test_root(root)
            .name("test_tree");
        let mut bt: BT<Ready> = bt.test_into_state();
        let map = convert_bt(&mut bt);
        // cond1 SUCCESS → a1
        assert_eq!(
            map.get(&(Node::Condition(id1.clone()), Status::Success)),
            Some(&Some(Node::Action(id2.clone())))
        );
        // cond1 FAILURE → cond2
        assert_eq!(
            map.get(&(Node::Condition(id1.clone()), Status::Failure)),
            Some(&Some(Node::Condition(id3.clone())))
        );
        // a1 FAILURE → cond2
        assert_eq!(
            map.get(&(Node::Action(id2.clone()), Status::Failure)),
            Some(&Some(Node::Condition(id3.clone())))
        );
        // cond2 SUCCESS → a2
        assert_eq!(
            map.get(&(Node::Condition(id3.clone()), Status::Success)),
            Some(&Some(Node::Action(id4.clone())))
        );
        // cond2 FAILURE → a3
        assert_eq!(
            map.get(&(Node::Condition(id3.clone()), Status::Failure)),
            Some(&Some(Node::Action(id5.clone())))
        );
        // a2 FAILURE → a3
        assert_eq!(
            map.get(&(Node::Action(id4.clone()), Status::Failure)),
            Some(&Some(Node::Action(id5.clone())))
        );
        // a3 FAILURE → None
        assert_eq!(map[&(Node::Action(id5.clone()), Status::Failure)], None);
        // All actions end cycle
        for id in [&id2, &id4, &id5] {
            assert_eq!(map[&(Node::Action(id.clone()), Status::Success)], None);
        }
    }

}