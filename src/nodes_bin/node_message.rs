use crate::nodes_bin::{node_error::NodeError, node::Node, node_status::Status};

// Result of listening to the current action and all active conditions
#[derive(Debug)]
pub(crate) enum FutResult {
    CurrentNode(bool),
    Condition(Node, bool),
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) enum ChildMessage {
    Start,
    Stop,
    Kill,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) enum ParentMessage {
    Status(Status),
    Poison(NodeError),
    Killed,
}