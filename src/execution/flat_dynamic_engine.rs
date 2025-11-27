use futures::future::select_all;
use futures::FutureExt;
use log::{trace, warn};

use crate::bt::Ready;
use crate::execution::engine_factory::Engine;
use crate::execution::flat_static_engine::FutureVec;
use crate::execution::process_comms::ProcessComms;
use crate::execution::traversal::search_next;
use crate::nodes_bin::node::Node;
use crate::nodes_bin::process_handle::ProcessHandle;
use crate::{BT, execution::traversal::search_start, nodes_bin::{node_message::{ChildMessage, FutResult, ParentMessage}, node_status::Status}};

pub(crate) struct FlatDynamicEngine {
    current_node: Node,
    current_trace: Vec<Node>,
    active_conditions: Vec<(Node, Vec<Node>)>,
    comms: ProcessComms,
}

impl FlatDynamicEngine {
    pub(crate) fn new(tree: &BT<Ready>) -> FlatDynamicEngine {
        let current_trace = search_start(&tree);
        let current_node = current_trace
            .last()
            .cloned()
            .expect("No initial node found for behavior tree"); //TODO both engines expect an initial node to be findable: check if tree is valid before doing this

        Self {
            current_node,
            current_trace,
            active_conditions: vec![],
            comms: ProcessComms::new(tree.map.clone()),
        }
    }

    async fn handle_current_node_finished(&mut self, status: bool) -> Option<bool>{
        self.current_trace = self.lookup_next(status);

        let Some(next_node) = self.current_trace.last().cloned() else {
            // The tree is finished
            self.kill_running().await;
            return Some(status);
        };

        // If the previous node was a condition, keep monitoring it
        if let Node::Condition(_) = self.current_node {
            self.active_conditions.push((self.current_node.clone(), self.current_trace.clone()));
        }

        self.current_node = next_node;
        None
    }

    async fn handle_condition_trigger(&mut self, status: bool, index: usize) -> Option<bool> {
        self.stop_conditions_after_idx(index).await;

        let (_, cond_trace) = self.active_conditions[index].clone(); // TODO watch out with index < len
        self.current_trace = search_next(cond_trace, &status.into());

        let Some(next_node) = self.current_trace.last().cloned() else {
            // The tree is finished
            self.kill_running().await;
            return Some(status);
        };

        self.current_node = next_node;
        None
    }

    fn lookup_next(&self, status: bool) -> Vec<Node>{
        search_next(self.current_trace.clone(), &status.into())
    }

    async fn stop_conditions_after_idx(&mut self, idx: usize) {
        for (condition,_) in self.active_conditions.split_off(idx + 1) {
            let Some(id) = condition.get_id() else {
                panic!("Unexpected node type");
            };
            // TODO: Handle Result here
            let _ = self.comms.send(id, ChildMessage::Stop).await;
        }
    }

    async fn start_current_node(&mut self) {
        let Some(id) = self.current_node.get_id() else {
            panic!("Unexpected node type");
        };
        if let Err(err) = self.comms.send(id, ChildMessage::Start).await {
            panic!("{:?} gave error {:?}", self.current_node, err);
        }
    }

    fn build_listener_futures<'a>(&'a mut self) -> FutureVec<'a>{
        let mut futures = vec![];

        // Futures for all active conditions
        for (cond,_) in self.active_conditions.clone().iter_mut() {
            let Some(id) = cond.get_id() else {
                panic!("Unexpected node type");
            };
            let handle = self.comms.get_handle(id).expect("No process found!");
            futures.push(Self::run_condition(cond.clone(), handle.clone()).boxed());
        }

        // Future for current action
        futures.push(self.run_current_node().boxed());
        futures
    }

    async fn run_condition(node: Node, mut handle: ProcessHandle) -> FutResult{
        loop {
            match handle.listen().await {
                Ok(msg) => {
                    match msg {
                        ParentMessage::Status(Status::Success) => return FutResult::Condition(node.clone(), true),
                        ParentMessage::Status(Status::Failure) => return FutResult::Condition(node.clone(), false),
                        _ => {} // Other messages should not be possible
                    }
                },
                Err(err) => {
                    warn!("{:?} has error {:?}", node, err)
                },
            }
        }
    }

    async fn run_current_node(&mut self) -> FutResult {
        let node = self.current_node.clone();
        let Some(id) = node.get_id() else {
            panic!("Unexpected node type");
        };
        let handle = self.comms.get_handle(id).expect("No process found!");
        loop {
            match handle.listen().await {
                Ok(msg) => {
                    if let Some(res) = Self::process_parent_message(node.clone(), msg) {
                        return FutResult::CurrentNode(res)
                    }
                },
                Err(err) => {
                    warn!("{:?} has error {:?}", self.current_node, err);
                    return FutResult::CurrentNode(false);
                },
            }
        }
    }

    fn process_parent_message(node: Node, msg: ParentMessage) -> Option<bool>{
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
                warn!("{:?} is poisoned with error: {:?}", node, err);
                Some(false)
            },
            ParentMessage::Killed => {
                warn!("{:?} has been killed", node);
                Some(false)
            },
        }
    }

    async fn kill_running(&mut self) {
        let Some(id) = self.current_node.get_id() else {
            panic!("Unexpected node type");
        };
        let _ = self.comms.send(id, ChildMessage::Kill).await;

        for (con,_) in self.active_conditions.clone() {
            let Some(id) = con.get_id() else {
                panic!("Unexpected node type");
            };
            let _ = self.comms.send(id, ChildMessage::Kill).await;
        }
    }
}

impl Engine for FlatDynamicEngine {
    async fn run(&mut self) -> bool {
        loop {
            self.start_current_node().await;

            let futures: FutureVec = self.build_listener_futures();
            let (result, index, _) = select_all(futures).await;
            trace!("Future with index {:?} returned: {:?}", index,result);

            if let Some(res) = match result {
                // Current node finished
                FutResult::CurrentNode(res) => self.handle_current_node_finished(res).await,
                // Previous condition switched
                FutResult::Condition(_, status) => self.handle_condition_trigger(status, index).await,
            } {
                return res;
            }
        }
    }
}