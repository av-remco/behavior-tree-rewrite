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
    use crate::nodes_bin::node::Node;
    use crate::nodes_bin::process_handle::ProcessHandle;
    use crate::nodes_bin::node_status::Status;
    use crate::{BT, Condition, Failure, Success, Wait};
    use logtest::Logger;

    // * Tests for search_down()
    #[tokio::test]
    async fn test_auto_failure() {
        let mut map = HashMap::new();
        let action1 = Failure::new();
        let id1 = "1".to_string();

        map.insert(id1.clone(), action1);

        let root = Node::Action(id1);
        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            root,                  // action visited
        ]);
    }

    #[tokio::test]
    async fn test_auto_success() {
        let mut map = HashMap::new();

        let action1 = Success::new();
        let id1 = "1".to_string();

        map.insert(id1.clone(), action1);

        let root = Node::Action(id1.clone());
        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![root]);
    }

    #[tokio::test]
    async fn test_condition_true_stops_at_condition() {
        let mut map = HashMap::new();

        let id1 = "cond1".to_string();
        let cond = Condition::new(&id1, Handle::new(5), |x| x > 0);

        map.insert(id1.clone(), cond);

        let root = Node::Condition(id1.clone());
        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![root]);
    }

    #[tokio::test]
    async fn test_sequence_hits_first_action() {
        let mut map = HashMap::new();

        let a1 = Success::new();
        let a2 = Success::new();

        let id1 = "a1".to_string();
        let id2 = "a2".to_string();

        map.insert(id1.clone(), a1);
        map.insert(id2.clone(), a2);

        let root = Node::Sequence(vec![
            Node::Action(id1.clone()),
            Node::Action(id2.clone()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            root.clone(),
            Node::Action(id1),
        ]);
    }

    #[tokio::test]
    async fn test_sequence_condition_stops_sequence() {
        let mut map = HashMap::new();

        let cond = Condition::new("cond", Handle::new(0), |x| x > 0);
        let a2 = Success::new();

        map.insert("cond".into(), cond);
        map.insert("a2".into(), a2);

        let root = Node::Sequence(vec![
            Node::Condition("cond".into()),
            Node::Action("a2".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            root.clone(),
            Node::Condition("cond".into()),
        ]);
    }

    #[tokio::test]
    async fn test_fallback_hits_first_action() {
        let mut map = HashMap::new();

        let fail1 = Failure::new();
        let succ = Success::new();

        map.insert("fail1".into(), fail1);
        map.insert("succ".into(), succ);

        let root = Node::Fallback(vec![
            Node::Action("fail1".into()),
            Node::Action("succ".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            root.clone(),
            Node::Action("fail1".into()),
        ]);
    }

    #[tokio::test]
    async fn test_fallback_condition_as_first_child() {
        let mut map = HashMap::new();

        let cond = Condition::new("cond_fb", Handle::new(0), |x| x > 0);
        let succ = Success::new();

        map.insert("cond_fb".into(), cond);
        map.insert("a2".into(), succ);

        let root = Node::Fallback(vec![
            Node::Condition("cond_fb".into()),
            Node::Action("a2".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            root.clone(),
            Node::Condition("cond_fb".into()),
        ]);
    }

    #[tokio::test]
    async fn test_nested_sequence_and_fallback() {
        let mut map = HashMap::new();

        let cond = Condition::new("cond_nested", Handle::new(1), |x| x > 0);
        let fail = Failure::new();
        let act = Success::new();

        map.insert("cond_nested".into(), cond);
        map.insert("fail".into(), fail);
        map.insert("act".into(), act);

        let fb = Node::Fallback(vec![
            Node::Action("fail".into()),
            Node::Action("act".into()),
        ]);

        let root = Node::Sequence(vec![
            Node::Condition("cond_nested".into()),
            fb.clone(),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            root.clone(),
            Node::Condition("cond_nested".into()),
        ]);
    }

    #[tokio::test]
    async fn test_fallback_sequence_condition_then_action() {
        let mut map = HashMap::new();

        let cond = Condition::new("cond1", Handle::new(3), |x| x > 0);
        let a1 = Success::new();
        let a2 = Success::new();

        map.insert("cond1".into(), cond);
        map.insert("a1".into(), a1);
        map.insert("a2".into(), a2);

        let seq = Node::Sequence(vec![
            Node::Condition("cond1".into()),
            Node::Action("a1".into()),
        ]);

        let root = Node::Fallback(vec![
            seq.clone(),
            Node::Action("a2".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let trace = search_start(&bt);

        assert_eq!(trace, vec![
            root.clone(),
            seq.clone(),
            Node::Condition("cond1".into()),
        ]);
    }

    // ---------- search_next tests ----------

    #[tokio::test]
    async fn test_search_next_sequence_continue() {
        let mut map = HashMap::new();

        let cond = Condition::new("cond1", Handle::new(1), |x| x > 0);
        let a1 = Success::new();
        let a2 = Success::new();

        map.insert("cond1".into(), cond);
        map.insert("a1".into(), a1);
        map.insert("a2".into(), a2);

        let root = Node::Sequence(vec![
            Node::Condition("cond1".into()),
            Node::Action("a1".into()),
            Node::Action("a2".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![
            root.clone(),
            Node::Condition("cond1".into()),
        ]);

        let next = search_next(start.clone(), &Status::Success);

        assert_eq!(next, vec![
            root.clone(),
            Node::Action("a1".into()),
        ]);
    }

    #[tokio::test]
    async fn test_search_next_sequence_stops_on_failure() {
        let mut map = HashMap::new();

        let cond = Condition::new("cond1", Handle::new(10), |x| x == 0);
        let a1 = Success::new();

        map.insert("cond1".into(), cond);
        map.insert("a1".into(), a1);

        let root = Node::Sequence(vec![
            Node::Condition("cond1".into()),
            Node::Action("a1".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![
            root.clone(),
            Node::Condition("cond1".into()),
        ]);

        let next = search_next(start.clone(), &Status::Failure);

        assert_eq!(next, vec![]);
    }

    #[tokio::test]
    async fn test_search_next_fallback_continue_on_failure() {
        let mut map = HashMap::new();

        let cond = Condition::new("c1", Handle::new(3), |x| x == 0);
        let a1 = Success::new();

        map.insert("c1".into(), cond);
        map.insert("a1".into(), a1);

        let root = Node::Fallback(vec![
            Node::Condition("c1".into()),
            Node::Action("a1".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![
            root.clone(),
            Node::Condition("c1".into()),
        ]);

        let next = search_next(start.clone(), &Status::Failure);

        assert_eq!(next, vec![
            root.clone(),
            Node::Action("a1".into()),
        ]);
    }

    #[tokio::test]
    async fn test_search_next_fallback_stop_on_success() {
        let mut map = HashMap::new();

        let cond = Condition::new("c1", Handle::new(2), |x| x > 0);
        let a1 = Success::new();

        map.insert("c1".into(), cond);
        map.insert("a1".into(), a1);

        let root = Node::Fallback(vec![
            Node::Condition("c1".into()),
            Node::Action("a1".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![
            root.clone(),
            Node::Condition("c1".into()),
        ]);

        let next = search_next(start.clone(), &Status::Success);

        assert_eq!(next, vec![]);
    }

    #[tokio::test]
    async fn test_search_next_nested() {
        let mut map = HashMap::new();

        let cond = Condition::new("c1", Handle::new(5), |x| x < 0);
        let a1 = Success::new();
        let a2 = Success::new();

        map.insert("c1".into(), cond);
        map.insert("a1".into(), a1);
        map.insert("a2".into(), a2);

        let seq = Node::Sequence(vec![
            Node::Condition("c1".into()),
            Node::Action("a1".into()),
        ]);

        let root = Node::Fallback(vec![
            seq.clone(),
            Node::Action("a2".into()),
        ]);

        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let start = search_start(&bt);
        assert_eq!(start, vec![
            root.clone(),
            seq.clone(),
            Node::Condition("c1".into()),
        ]);

        let next = search_next(start.clone(), &Status::Failure);

        assert_eq!(next, vec![
            root.clone(),
            Node::Action("a2".into()),
        ]);
    }

    #[tokio::test]
    async fn test_search_next_on_root_returns_empty() {
        let mut map = HashMap::new();

        let a1 = Success::new();
        map.insert("a1".into(), a1);

        let root = Node::Action("a1".into());
        let bt = BT::new().test_insert_map(map).test_root(root.clone()).name("test_tree");

        let fst_trace = search_start(&bt);
        let snd_trace = search_next(fst_trace.clone(), &Status::Success);
        let trd_trace = search_next(fst_trace.clone(), &Status::Failure);

        assert_eq!(fst_trace, vec![root.clone()]);
        assert_eq!(snd_trace, Vec::<Node>::new());
        assert_eq!(trd_trace, Vec::<Node>::new());
    }

}