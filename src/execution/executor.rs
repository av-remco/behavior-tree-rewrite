use std::pin::Pin;

use futures::future::select_all;
use futures::{Future, FutureExt};
use log::{trace, warn};

use crate::{BT, bt::Processing, conversion::converter::{BehaviorTreeMap, convert_bt}, execution::traversal::search_start, nodes_bin::{node::NodeType, node_handle::NodeHandle, node_message::{ChildMessage, FutResult, ParentMessage}, node_status::Status}};

type FutureVec<'a> = Vec<Pin<Box<dyn Future<Output = FutResult> + Send + 'a>>>;

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
            self.start_current_node();
            let futures: FutureVec = self.build_listener_futures();

            // Wait for first message
            let (result, index, _) = select_all(futures).await;
            trace!("Future with index {:?} returned: {:?}", index,result);

            if let Some(res) = match result {
                // Current node finished
                FutResult::CurrentNode(res) => self.process_return_value(res).await,
                // Previous condition switched
                FutResult::Condition(node, status) => self.process_condition(node, status, index).await,
            } {
                return res;
            }
        }
    }

    fn map(&self, node: NodeHandle, status: bool) -> Option<NodeHandle>{
        self.map
            .get(&(node, status.into()))
            .cloned()
            .flatten()
    }

    async fn stop_next_conditions(&mut self, index: usize) {
        for mut condition in self.active_conditions.split_off(index + 1) {
            condition.stop().await
        }
    }

    async fn process_condition(&mut self, node: NodeHandle, status: bool, index: usize) -> Option<bool> {
        self.current_node.stop().await;
        self.stop_next_conditions(index).await;

        let Some(next_node) = self.map(node, status) else {
            // The tree is finished
            self.kill().await;
            return Some(status);
        };

        self.current_node = next_node;
        None
    }

    async fn process_return_value(&mut self, status: bool) -> Option<bool>{
        let Some(next_node) = self.map(self.current_node.clone(), status) else {
            // The tree is finished
            self.kill().await;
            return Some(status);
        };

        // If the previous node was a condition, keep monitoring it
        if let NodeType::Condition = self.current_node.element {
            self.active_conditions.push(self.current_node.clone());
        }

        self.current_node = next_node;
        None
    }

    fn build_listener_futures<'a>(&'a mut self) -> FutureVec<'a>{
        let mut futures = vec![];

        // Futures for all active conditions
        for handle in self.active_conditions.clone().iter_mut() {
            futures.push(Self::run_condition(handle.clone()).boxed());
        }

        // Future for current action
        futures.push(self.run_current_node().boxed());
        futures
    }

    fn start_current_node(&self) {
        if let Err(err) = self.current_node.send(ChildMessage::Start) {
            panic!("{:?} {:?} gave error {:?}", self.current_node.element, self.current_node.name.clone(), err);
        }
    }

    async fn run_condition(mut node: NodeHandle) -> FutResult{
        loop {
            match node.listen().await {
                Ok(msg) => {
                    match msg {
                        ParentMessage::Status(Status::Success) => return FutResult::Condition(node.clone(), true),
                        ParentMessage::Status(Status::Failure) => return FutResult::Condition(node.clone(), false),
                        _ => {} // Other messages should not be possible
                    }
                },
                Err(err) => {
                    warn!("{:?} {:?} has error {:?}", node.element, node.name, err)
                },
            }
        }
    }

    async fn run_current_node(&mut self) -> FutResult {
        loop {
            match self.current_node.listen().await {
                Ok(msg) => {
                    if let Some(res) = self.process_parent_message(msg) {
                        return FutResult::CurrentNode(res)
                    }
                },
                Err(err) => {
                    warn!("{:?} {:?} has error {:?}", self.current_node.element, self.current_node.name.clone(), err);
                    return FutResult::CurrentNode(false);
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
            },
        }
    }

    async fn kill(&mut self) {
        self.current_node.kill().await;

        for mut node in self.active_conditions.clone() {
            node.kill().await;
        }
    }
}