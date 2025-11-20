use tokio::sync::broadcast::Receiver;

use crate::nodes_bin::{node_error::NodeError, node_status::Status};

#[derive(Debug)]
pub enum FutResponse {
    Parent(ChildMessage, Receiver<ChildMessage>),
    Child(usize, ParentMessage, Receiver<ParentMessage>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum ChildMessage {
    Start,
    Stop,
    Kill,
}

impl ChildMessage {
    pub fn is_kill(&self) -> bool {
        *self == ChildMessage::Kill
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum ParentMessage {
    RequestStart,
    Status(Status),
    Poison(NodeError),
    Killed,
}