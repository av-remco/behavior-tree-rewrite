use anyhow::Error;

use crate::{BT, bt::Processing, conversion::converter::{BehaviorTreeMap, convert_bt}, execution::traversal::search_start, nodes_bin::node_handle::NodeHandle};

// TODO: Not how Factory pattern works, but good enough for one executor
pub struct ExecutorFactory {}

impl ExecutorFactory {
    pub(crate) fn create(&self, tree: &BT<Processing>) -> Executor {
        Executor::new(tree)
    }
}

pub struct Executor {
    current_trace: Vec<NodeHandle>,
    map: BehaviorTreeMap,
}

impl Executor {
    pub(crate) fn new(tree: &BT<Processing>) -> Executor {
        let current_trace = search_start(tree);
        let map = convert_bt(tree);
        Self {
            current_trace,
            map,
        }
    }

    pub(crate) async fn execute(&self) -> bool {
        true
    }
}