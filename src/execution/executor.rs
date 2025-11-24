use log::warn;

use crate::{BT, bt::Processing, conversion::converter::{BehaviorTreeMap, convert_bt}, execution::traversal::search_start, nodes_bin::{node::NodeType, node_handle::NodeHandle, node_message::{ChildMessage, ParentMessage}, node_status::Status}};

// TODO: Not how Factory pattern works, but good enough for one executor
pub struct ExecutorFactory {}

impl ExecutorFactory {
    pub(crate) fn create(&self, tree: &BT<Processing>) -> Executor {
        Executor::new(tree)
    }
}

pub struct Executor {
    current_node: NodeHandle,
    map: BehaviorTreeMap,
}

impl Executor {
    pub(crate) fn new(tree: &BT<Processing>) -> Executor {
        let current_node = search_start(tree)
            .last()
            .cloned()
            .expect("No initial node found for behavior tree");
        
        let map = convert_bt(tree);
        Self {
            current_node,
            map,
        }
    }

    pub(crate) async fn execute(&mut self) -> bool {
        loop {
            let res = self.execute_node().await;

            if let Some(node) = self.map.get(&(self.current_node.clone(), res.into())).and_then(|x| x.clone()) {
                self.current_node = node;
            } else {
                self.kill();
                return res;
            }
        }
    }

    async fn execute_node(&mut self) -> bool {
        if let Err(err) = self.current_node.send(ChildMessage::Start) {
            warn!("Node {:?} {:?} exited with error {:?}", self.current_node.element, self.current_node.name.clone(), err);
            return false;
        }
        loop {
            match self.current_node.listen().await {
                Ok(msg) => {
                    if let Some(res) = self.process_parent_message(msg) {
                        return res
                    }
                },
                Err(err) => {
                    warn!("Node {:?} {:?} has error {:?}", self.current_node.element, self.current_node.name.clone(), err);
                    return false;
                },
            }
        }
    }

    fn process_parent_message(&self, msg: ParentMessage) -> Option<bool>{
        match msg {
            ParentMessage::RequestStart => {
                warn!("Node {:?} {:?} send unexpected RequestStart", self.current_node.element, self.current_node.name.clone());
                Some(false)
            },
            ParentMessage::Status(status) => match status {
                    Status::Success => {
                        Some(true)
                    },
                    Status::Failure => {
                        Some(false)
                    },
                    _ => None
                },
            ParentMessage::Poison(err) => {
                warn!("Node {:?} {:?} is poisoned with error: {:?}", self.current_node.element, self.current_node.name.clone(), err);
                Some(false)
            },
            ParentMessage::Killed => {
                warn!("Node {:?} {:?} has been killed", self.current_node.element, self.current_node.name.clone());
                Some(false)
            }, // This should not occur
        }
    }

    fn kill(&self) {
        if let Err(err) = self.current_node.send(ChildMessage::Kill) {
            warn!("Node {:?} {:?} exited with error {:?}", self.current_node.element, self.current_node.name.clone(), err);
        }
    }
}