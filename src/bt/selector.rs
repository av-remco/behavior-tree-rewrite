use anyhow::Result;
use futures::future::select_all;
use futures::{Future, FutureExt};

use std::pin::Pin;
use tokio::sync::broadcast::{channel, Receiver, Sender};

use super::handle::NodeError;
use crate::NodeType;
use crate::bt::handle::{ChildMessage, FutResponse, Node, NodeHandle, ParentMessage, Status};
use crate::bt::CHANNEL_SIZE;

// Simplify complex type
type FutVec = Vec<Pin<Box<dyn Future<Output = Result<FutResponse, NodeError>> + Send>>>;

// Prevent typo errors in booleans by using explicit types
pub struct Sequence {
    children: Vec<NodeHandle>,
}

impl Sequence {
    pub fn new(
        mut children: Vec<NodeHandle>,
    ) -> NodeHandle {
        // TODO: Channels are useless, but required
        let (parent_tx, parent_rx) = channel(CHANNEL_SIZE);
        let (child_tx, child_rx) = channel(CHANNEL_SIZE);

        let child_names = children.iter().map(|x| x.name.clone()).collect();
        let child_ids = children.iter().map(|x| x.id.clone()).collect();
        let mut handles = vec![];
        for child in children.iter_mut() {
            handles.append(&mut child.take_handles());
        }

        NodeHandle::new(
            parent_tx,
            child_rx,
            NodeType::Sequence,
            "Sequence",
            child_names,
            child_ids,
            handles,
        )
    }
}

pub struct Fallback {
    children: Vec<NodeHandle>,
}

impl Fallback {
    pub fn new(
        mut children: Vec<NodeHandle>,
    ) -> NodeHandle {
        // TODO: Channels are useless, but required
        let (parent_tx, parent_rx) = channel(CHANNEL_SIZE);
        let (child_tx, child_rx) = channel(CHANNEL_SIZE);

        let child_names = children.iter().map(|x| x.name.clone()).collect();
        let child_ids = children.iter().map(|x| x.id.clone()).collect();
        let mut handles = vec![];
        for child in children.iter_mut() {
            handles.append(&mut child.take_handles());
        }

        NodeHandle::new(
            parent_tx,
            child_rx,
            NodeType::Fallback,
            "Fallback",
            child_names,
            child_ids,
            handles,
        )
    }
}