use std::pin::Pin;
use futures::future::select_all;
use futures::{Future};

use futures::FutureExt;
use log::{debug, trace, warn};
use tokio_tungstenite::tungstenite::http::status;

use crate::nodes_bin::node_error::NodeError;
use crate::nodes_bin::node_message::FutResponse;
use crate::{BT, bt::Processing, conversion::converter::{BehaviorTreeMap, convert_bt}, execution::traversal::search_start, nodes_bin::{node::NodeType, node_handle::NodeHandle, node_message::{ChildMessage, ParentMessage}, node_status::Status}};

// TODO: Not how Factory pattern works, but good enough for one executor
pub struct ExecutorFactory {}

impl ExecutorFactory {
    pub(crate) fn create(&self, tree: &BT<Processing>) -> Executor {
        Executor::new(tree)
    }
}

#[derive(Debug)]
enum FutResult {
    Result(bool),
    Node(NodeHandle, Status),
}

pub struct Executor {
    current_node: NodeHandle,
    map: BehaviorTreeMap,
    active_conditions: Vec<NodeHandle>,
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
            active_conditions: vec![],
        }
    }

    pub(crate) async fn execute(&mut self) -> bool {
        loop {
            let mut futures = vec![];

            for handle in self.active_conditions.clone().iter_mut() {
                futures.push(Self::monitor_condition(handle.clone()).boxed());
            }
            futures.push(self.execute_node().boxed());


            let (result, index, _) = select_all(futures).await; // Listen out all actions
            debug!("Future with index {:?} returned: {:?}", index,result);

            match result {
                FutResult::Result(res) => {
                    let map_result = self
                        .map
                        .get(&(self.current_node.clone(), res.into()))
                        .cloned()
                        .flatten();

                    // If no next node in the map, the behavior tree is finished
                    let Some(next_node) = map_result else {
                        self.kill().await;
                        return res;
                    };

                    // If the previous node was a condition, keep monitoring it
                    if let NodeType::Condition = self.current_node.element {
                        self.active_conditions.push(self.current_node.clone());
                    }

                    self.current_node = next_node;
                },
                FutResult::Node(node, status) => {
                    if let Err(err) = self.current_node.send(ChildMessage::Stop) {
                        warn!("{:?} {:?} has error {:?}", self.current_node.element, self.current_node.name, err)
                    }
                    if let Err(err) = self.current_node.listen().await {
                        warn!("{:?} {:?} has error {:?}", self.current_node.element, self.current_node.name, err)
                    }
                    
                    let map_result = self
                        .map
                        .get(&(node, status))
                        .cloned()
                        .flatten();

                    // If no next node in the map, the behavior tree is finished
                    let Some(next_node) = map_result else {
                        self.kill().await;
                        if let Some(res) = status.into() {
                            return res;
                        } else {
                            panic!("Unexpected Status {:?}", status);
                        }
                    };

                    self.current_node = next_node;

                    let to_stop = self.active_conditions.split_off(index + 1);
                    for mut handle in to_stop {
                        if let Err(err) = handle.send(ChildMessage::Stop) {
                            warn!("{:?} {:?} has error {:?}", self.current_node.element, self.current_node.name, err)
                        }
                        if let Err(err) = handle.listen().await {
                            warn!("{:?} {:?} has error {:?}", self.current_node.element, self.current_node.name, err)
                        }
                    }
                },
            }            
        }
    }

    async fn monitor_condition(mut node: NodeHandle) -> FutResult{
        loop {
            match node.listen().await {
                Ok(msg) => {
                    match msg {
                        ParentMessage::Status(Status::Success) => return FutResult::Node(node.clone(), Status::Success),
                        ParentMessage::Status(Status::Failure) => return FutResult::Node(node.clone(), Status::Failure),
                        _ => {} // Other messages should not be possible
                    }
                },
                Err(err) => {
                    warn!("{:?} {:?} has error {:?}", node.element, node.name, err)
                },
            }
        }
    }

    async fn execute_node(&mut self) -> FutResult {
        if let Err(err) = self.current_node.send(ChildMessage::Start) {
            warn!("{:?} {:?} gave error {:?}", self.current_node.element, self.current_node.name.clone(), err);
            return FutResult::Result(false);
        }

        loop {
            match self.current_node.listen().await {
                Ok(msg) => {
                    if let Some(res) = self.process_parent_message(msg) {
                        return FutResult::Result(res)
                    }
                },
                Err(err) => {
                    warn!("{:?} {:?} has error {:?}", self.current_node.element, self.current_node.name.clone(), err);
                    return FutResult::Result(false);
                },
            }
        }
    }

    fn process_parent_message(&self, msg: ParentMessage) -> Option<bool>{
        match msg {
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
                warn!("{:?} {:?} is poisoned with error: {:?}", self.current_node.element, self.current_node.name.clone(), err);
                Some(false)
            },
            ParentMessage::Killed => {
                warn!("{:?} {:?} has been killed", self.current_node.element, self.current_node.name.clone());
                Some(false)
            }, // This should not occur
        }
    }

    async fn kill(&mut self) {
        self.current_node.kill().await;

        for mut node in self.active_conditions.clone() {
            node.kill().await;
        }
    }
}