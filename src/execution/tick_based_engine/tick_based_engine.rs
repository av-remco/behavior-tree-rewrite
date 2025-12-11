use std::time::Duration;

use actify::Handle;
use futures::{FutureExt, future::select_all};
use log::{trace, warn};
use tokio::time::sleep;

use crate::{execution::{engine_factory::Engine, process_comms::{FutureVec, ProcessComms}}, nodes_bin::{node::Node, node_message::{ChildMessage, FutResult, ParentMessage}, node_status::Status}};

const MS_BETWEEN_TICKS: u64 = 10; // in ms

pub(crate) struct TickBasedEngine {
    root: Node,
    current_node: Node,
    current_status: Handle<Status>,
    comms: ProcessComms,
}

impl TickBasedEngine {
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

        // Future for running tick
        futures.push(Self::tick_tree(self.root.clone(), self.current_node.clone(), self.current_status.clone()).boxed());

        // Future for current action
        futures.push(self.run_current_node().boxed());
        futures
    }

    async fn tick_tree(root: Node, current_node: Node, current_status: Handle<Status>) -> FutResult {
        let mut ticked_action: Node;
        loop {
            ticked_action = Self::get_ticked_action(root.clone(), current_node.clone(), current_status.clone()).await;
            if ticked_action != current_node {
                break;
            }
            sleep(Duration::from_millis(MS_BETWEEN_TICKS)).await;
        }
        FutResult::Condition(ticked_action, true)
    }

    async fn get_ticked_action(root: Node, current_node: Node, current_status: Handle<Status>) -> Node {
        match root {
            Node::Action(_) => ,
            Node::Condition(_) => todo!(),
            Node::Sequence(nodes) => todo!(),
            Node::Fallback(nodes) => todo!(),
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

    async fn handle_current_node_finished(&mut self, status: bool){
        self.current_status.set_if_changed(status.into()).await;
    }

    async fn kill_running(&mut self) {
        let Some(id) = self.current_node.get_id() else {
            panic!("Unexpected node type");
        };
        let _ = self.comms.send(id, ChildMessage::Kill).await;
    }
}

impl Engine for TickBasedEngine {
    async fn run(&mut self) -> bool {
        loop {
            self.start_current_node().await;

            let futures: FutureVec = self.build_listener_futures();
            let (result, index, _) = select_all(futures).await; // TODO check if futures != empty, select_all panics
            trace!("Future with index {:?} returned: {:?}", index,result);

            match result {
                // Current node finished
                FutResult::CurrentNode(res) => self.handle_current_node_finished(res).await,
                // Previous condition switched: node is the new action
                FutResult::Condition(node, _) => panic!("Tick-based engine does not create condition futures!"),
            }
        }
    }
}
