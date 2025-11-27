#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use std::{collections::HashMap, time::Duration};
    use actify::Handle;
    use tokio::time::sleep;
    use crate::{BT, Condition, Failure, Success, Wait, bt::Ready, execution::engine_factory::Engines, logging::load_logger, nodes::action::mocking::MockAction, nodes_bin::{node::Node, node_status::Status}};


    // Test for each engine type
    const ENGINE: Engines = Engines::Dynamic;

    #[tokio::test]
    async fn test_execute_simple_success() {
        let mut map = HashMap::new();
        let action = Success::new();
        let id = "a1".to_string();
        map.insert(id.clone(), action);

        let root = Node::Action(id);
        let bt = BT::new().test_insert_map(map).test_root(root).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_simple_failure() {
        let mut map = HashMap::new();
        let action = Failure::new();
        let id = "a1".to_string();
        map.insert(id.clone(), action);

        let root = Node::Action(id);
        let bt = BT::new().test_insert_map(map).test_root(root).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_condition_true() {
        let mut map = HashMap::new();
        let id = "cond".to_string();
        let cond = Condition::new("cond_true", Handle::new(10), |x| x > 0);
        map.insert(id.clone(), cond);

        let root = Node::Condition(id);
        let bt = BT::new().test_insert_map(map).test_root(root).set_engine(ENGINE).name("test_tree");

        let bt = bt.test_into_state().run().await;
        assert_eq!(bt.result(), true);
    }

    #[tokio::test]
    async fn test_execute_condition_false() {
        let mut map = HashMap::new();
        let id = "cond".to_string();
        let cond = Condition::new("cond_false", Handle::new(0), |x| x > 5);
        map.insert(id.clone(), cond);

        let root = Node::Condition(id);
        let bt = BT::new().test_insert_map(map).test_root(root).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_sequence_all_success() {
        let mut map = HashMap::new();

        let a1 = Success::new();
        let a2 = Success::new();
        let id1 = "a1".to_string();
        let id2 = "a2".to_string();
        map.insert(id1.clone(), a1);
        map.insert(id2.clone(), a2);

        let seq = Node::Sequence(vec![Node::Action(id1), Node::Action(id2)]);
        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_sequence_stops_on_failure() {
        let mut map = HashMap::new();

        let a1 = Success::new();
        let a2 = Failure::new();
        let id1 = "a1".to_string();
        let id2 = "a2".to_string();
        map.insert(id1.clone(), a1);
        map.insert(id2.clone(), a2);

        let seq = Node::Sequence(vec![Node::Action(id1), Node::Action(id2)]);
        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_fallback_first_success() {
        let mut map = HashMap::new();

        let s1 = Success::new();
        let f1 = Failure::new();
        let id1 = "s1".to_string();
        let id2 = "f1".to_string();
        map.insert(id1.clone(), s1);
        map.insert(id2.clone(), f1);

        let fb = Node::Fallback(vec![Node::Action(id1), Node::Action(id2)]);
        let bt = BT::new().test_insert_map(map).test_root(fb).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_fallback_second_success() {
        let mut map = HashMap::new();

        let f1 = Failure::new();
        let s1 = Success::new();
        let id1 = "f1".to_string();
        let id2 = "s1".to_string();
        map.insert(id1.clone(), f1);
        map.insert(id2.clone(), s1);

        let fb = Node::Fallback(vec![Node::Action(id1), Node::Action(id2)]);
        let bt = BT::new().test_insert_map(map).test_root(fb).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_execute_fallback_all_fail() {
        let mut map = HashMap::new();

        let f1 = Failure::new();
        let f2 = Failure::new();
        let id1 = "f1".to_string();
        let id2 = "f2".to_string();
        map.insert(id1.clone(), f1);
        map.insert(id2.clone(), f2);

        let fb = Node::Fallback(vec![Node::Action(id1), Node::Action(id2)]);
        let bt = BT::new().test_insert_map(map).test_root(fb).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_nested_sequence_fallback() {
        let mut map = HashMap::new();

        let idc = "cond".to_string();
        let idf1 = "f1".to_string();
        let ids1 = "s1".to_string();

        map.insert(idc.clone(), Condition::new("nested", Handle::new(0), |x| x > 0));
        map.insert(idf1.clone(), Failure::new());
        map.insert(ids1.clone(), Success::new());

        let fb = Node::Fallback(vec![Node::Action(idf1), Node::Action(ids1)]);
        let seq = Node::Sequence(vec![Node::Condition(idc), fb]);

        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), false);
    }

    #[tokio::test]
    async fn test_execute_wait_action() {
        let mut map = HashMap::new();

        let id = "wait".to_string();
        map.insert(id.clone(), Wait::new(Duration::from_millis(50)));

        let root = Node::Action(id);
        let bt = BT::new().test_insert_map(map).test_root(root).set_engine(ENGINE).name("test_tree");

        let result = bt.test_into_state().run().await;
        assert_eq!(result.result(), true);
    }

    #[tokio::test]
    async fn test_condition_interrupt() {
        let mut map = HashMap::new();

        let handle = Handle::new(1);

        let idc = "cond".to_string();
        let ida = "action".to_string();
        map.insert(idc.clone(), Condition::new("cond", handle.clone(), |x| x > 0));
        map.insert(ida.clone(), MockAction::new(1));

        let seq = Node::Sequence(vec![Node::Condition(idc), Node::Action(ida)]);
        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let (bt, _) = tokio::join!(
            bt.test_into_state().run(),
            async {
                sleep(Duration::from_millis(200)).await;
                handle.set(-1).await;
            }
        );

        assert_eq!(bt.result(), false);
    }

    #[tokio::test]
    async fn test_error_propagation_in_sequence() {
        let mut map = HashMap::new();

        let id1 = "a1".to_string();
        let ide = "e".to_string();
        let id2 = "a2".to_string();

        map.insert(id1.clone(), MockAction::new(1));
        map.insert(ide.clone(), MockAction::new_error(2));
        map.insert(id2.clone(), MockAction::new(3));

        let seq = Node::Sequence(vec![
            Node::Action(id1),
            Node::Action(ide),
            Node::Action(id2),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let bt = bt.test_into_state().run().await;
        assert_eq!(bt.result(), false);
    }

    #[tokio::test]
    async fn test_two_conditions_switching() {
        let mut map = HashMap::new();

        let h1 = Handle::new(1);
        let h2 = Handle::new(1);

        let id1 = "c1".to_string();
        let id2 = "c2".to_string();
        let ida = "act".to_string();

        map.insert(id1.clone(), Condition::new("cond1", h1.clone(), |x| x > 0));
        map.insert(id2.clone(), Condition::new("cond2", h2.clone(), |x| x > 0));
        map.insert(ida.clone(), MockAction::new(1));

        let seq = Node::Sequence(vec![
            Node::Condition(id1),
            Node::Condition(id2),
            Node::Action(ida),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let (bt, _, _) = tokio::join!(
            bt.test_into_state().run(),
            async {
                sleep(Duration::from_millis(200)).await;
                h2.set(0).await;
            },
            async {
                sleep(Duration::from_millis(200)).await;
                h1.set(0).await;
            }
        );

        assert_eq!(bt.result(), false);
    }

    #[tokio::test]
    async fn test_condition_fails_mid_sequence() {
        let mut map = HashMap::new();

        let h1 = Handle::new(1);
        let h2 = Handle::new(1);

        let id1 = "c1".to_string();
        let id2 = "c2".to_string();
        let ida = "act".to_string();

        map.insert(id1.clone(), Condition::new("cond1", h1.clone(), |x| x > 0));
        map.insert(id2.clone(), Condition::new("cond2", h2.clone(), |x| x > 0));
        map.insert(ida.clone(), MockAction::new(1));

        let seq = Node::Sequence(vec![
            Node::Condition(id1),
            Node::Condition(id2),
            Node::Action(ida),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let (bt, _) = tokio::join!(
            bt.test_into_state().run(),
            async {
                sleep(Duration::from_millis(300)).await;
                h2.set(0).await;
            }
        );

        assert_eq!(bt.result(), false);
    }

    #[tokio::test]
    async fn test_multiple_conditions_toggle() {
        let mut map = HashMap::new();

        let h1 = Handle::new(0);
        let h2 = Handle::new(0);
        let h3 = Handle::new(0);

        let id1 = "c1".to_string();
        let id2 = "c2".to_string();
        let id3 = "c3".to_string();
        let ida = "a".to_string();

        map.insert(id1.clone(), Condition::new("c1", h1.clone(), |x| x > 0));
        map.insert(id2.clone(), Condition::new("c2", h2.clone(), |x| x > 0));
        map.insert(id3.clone(), Condition::new("c3", h3.clone(), |x| x > 0));
        map.insert(ida.clone(), MockAction::new(1));

        let seq = Node::Fallback(vec![
            Node::Condition(id1),
            Node::Condition(id2),
            Node::Condition(id3),
            Node::Action(ida),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let (bt, _) = tokio::join!(
            bt.test_into_state().run(),
            async {
                sleep(Duration::from_millis(100)).await;
                h1.set(1).await;
                sleep(Duration::from_millis(100)).await;
                h2.set(1).await;
                sleep(Duration::from_millis(100)).await;
                h3.set(1).await;
            }
        );

        assert_eq!(bt.result(), true);
    }

    #[tokio::test]
    async fn test_stop_monitoring_condition() {
        let mut map = HashMap::new();

        let h1 = Handle::new(1);
        let h2 = Handle::new(0);

        let id1 = "c1".to_string();
        let id2 = "c2".to_string();
        let ida = "a".to_string();

        map.insert(id1.clone(), Condition::new("c1", h1.clone(), |x| x > 0));
        map.insert(id2.clone(), Condition::new("c2", h2.clone(), |x| x > 0));
        map.insert(ida.clone(), MockAction::new(1));

        let seq = Node::Fallback(vec![
            Node::Sequence(vec![
                Node::Condition(id1),
                Node::Condition(id2),
            ]),
            Node::Action(ida),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(seq).set_engine(ENGINE).name("test_tree");

        let (bt, _) = tokio::join!(
            bt.test_into_state().run(),
            async {
                sleep(Duration::from_millis(100)).await;
                h1.set(0).await;
                sleep(Duration::from_millis(100)).await;
                h2.set(1).await;
            }
        );

        assert_eq!(bt.result(), true);
    }

}