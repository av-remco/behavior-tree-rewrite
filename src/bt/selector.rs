use anyhow::Result;
use futures::Future;

use std::pin::Pin;
use tokio::sync::broadcast::channel;

use super::handle::NodeError;
use crate::NodeType;
use crate::bt::handle::{FutResponse, NodeHandle};
use crate::bt::CHANNEL_SIZE;

pub struct Sequence {
    children: Vec<NodeHandle>,
}

impl Sequence {
    pub fn new(
        children: Vec<NodeHandle>,
    ) -> NodeHandle {
        // TODO: Channels are useless, but required
        let (parent_tx, parent_rx) = channel(CHANNEL_SIZE);
        let (child_tx, child_rx) = channel(CHANNEL_SIZE);

        let child_names = children.iter().map(|x| x.name.clone()).collect();
        let child_ids = children.iter().map(|x| x.id.clone()).collect();

        NodeHandle::new(
            parent_tx,
            child_rx,
            NodeType::Sequence,
            "Sequence",
            child_names,
            child_ids,
            children,
        )
    }
}

pub struct Fallback {
    children: Vec<NodeHandle>,
}

impl Fallback {
    pub fn new(
        children: Vec<NodeHandle>,
    ) -> NodeHandle {
        // TODO: Channels are useless, but required
        let (parent_tx, parent_rx) = channel(CHANNEL_SIZE);
        let (child_tx, child_rx) = channel(CHANNEL_SIZE);

        let child_names = children.iter().map(|x| x.name.clone()).collect();
        let child_ids = children.iter().map(|x| x.id.clone()).collect();

        NodeHandle::new(
            parent_tx,
            child_rx,
            NodeType::Fallback,
            "Fallback",
            child_names,
            child_ids,
            children,
        )
    }
}