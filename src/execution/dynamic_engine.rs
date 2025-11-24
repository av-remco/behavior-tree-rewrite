use futures::future::select_all;
use futures::FutureExt;
use log::{trace, warn};

use crate::bt::Ready;
use crate::execution::executor_factory::Engine;
use crate::execution::flat_map_engine::FutureVec;
use crate::execution::traversal::search_next;
use crate::{BT, execution::traversal::search_start, nodes_bin::{node::NodeType, node_handle::NodeHandle, node_message::{ChildMessage, FutResult, ParentMessage}, node_status::Status}};

pub(crate) struct DynamicEngine<'a> {
    bt_ref: &'a BT<Ready>,
    current_trace: Vec<NodeHandle>,
    active_conditions: Vec<NodeHandle>,
}

impl<'a> DynamicEngine<'a> {
    pub(crate) fn new(tree: &'a BT<Ready>) -> DynamicEngine {
        Self {
            bt_ref: tree,
            current_trace: search_start(tree),
            active_conditions: vec![],
        }
    }

    async fn handle_current_node_finished(&mut self, status: bool) -> Option<bool>{
        let Some(next_node) = self.lookup_next(self.current_trace.clone(), status) else {
            // The tree is finished
            self.kill_running().await;
            return Some(status);
        };

        // If the previous node was a condition, keep monitoring it
        if let NodeType::Condition = self.current_trace.element {
            self.active_conditions.push(self.current_trace.clone());
        }

        self.current_trace = next_node;
        None
    }

    async fn handle_condition_trigger(&mut self, node: NodeHandle, status: bool, index: usize) -> Option<bool> {
        self.current_trace.stop().await;
        self.stop_conditions_after_idx(index).await;

        let Some(next_node) = self.lookup_next(node, status) else {
            // The tree is finished
            self.kill_running().await;
            return Some(status);
        };

        self.current_trace = next_node;
        None
    }

    fn lookup_next(&self, node: NodeHandle, status: bool) -> Vec<NodeHandle>{
        search_next(tree, self.current_trace, &status.into())
    }

    async fn stop_conditions_after_idx(&mut self, idx: usize) {
        for mut condition in self.active_conditions.split_off(idx + 1) {
            condition.stop().await
        }
    }

    fn start_current_node(&self) {
        if let Err(err) = self.current_trace.send(ChildMessage::Start) {
            panic!("{:?} {:?} gave error {:?}", self.current_trace.element, self.current_trace.name.clone(), err);
        }
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
            match self.current_trace.listen().await {
                Ok(msg) => {
                    if let Some(res) = self.process_parent_message(msg) {
                        return FutResult::CurrentNode(res)
                    }
                },
                Err(err) => {
                    warn!("{:?} {:?} has error {:?}", self.current_trace.element, self.current_trace.name.clone(), err);
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
                warn!("{:?} {:?} is poisoned with error: {:?}", self.current_trace.element, self.current_trace.name.clone(), err);
                Some(false)
            },
            ParentMessage::Killed => {
                warn!("{:?} {:?} has been killed", self.current_trace.element, self.current_trace.name.clone());
                Some(false)
            },
        }
    }

    async fn kill_running(&mut self) {
        self.current_trace.kill().await;

        for mut con in self.active_conditions.clone() {
            con.kill().await;
        }
    }
}

impl<'a> Engine for DynamicEngine<'a> {
    async fn run(&mut self) -> bool {
        loop {
            self.start_current_node();

            let futures: FutureVec = self.build_listener_futures();
            let (result, index, _) = select_all(futures).await;
            trace!("Future with index {:?} returned: {:?}", index,result);

            if let Some(res) = match result {
                // Current node finished
                FutResult::CurrentNode(res) => self.handle_current_node_finished(res).await,
                // Previous condition switched
                FutResult::Condition(node, status) => self.handle_condition_trigger(node, status, index).await,
            } {
                return res;
            }
        }
    }
}