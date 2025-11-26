use crate::nodes_bin::{node_error::NodeError, process_handle::ProcessHandle, node_status::Status};

#[derive(Debug)]
pub(crate) enum FutResult {
    CurrentNode(bool),
    Condition(ProcessHandle, bool),
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