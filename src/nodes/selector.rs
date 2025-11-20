use tokio::sync::broadcast::channel;

use crate::{bt::CHANNEL_SIZE, nodes_bin::{node::NodeType, node_handle::NodeHandle}};

pub struct Sequence {
    children: Vec<NodeHandle>,
    handles: Vec<NodeHandle>,
}

impl Sequence {
    pub fn new(
        mut children: Vec<NodeHandle>,
    ) -> NodeHandle {
        // TODO: Channels are useless, but required
        let (parent_tx, _) = channel(CHANNEL_SIZE);
        let (_, child_rx) = channel(CHANNEL_SIZE);

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
        let (parent_tx, _) = channel(CHANNEL_SIZE);
        let (_, child_rx) = channel(CHANNEL_SIZE);

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