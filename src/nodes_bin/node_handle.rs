use std::hash::Hash;

use anyhow::Result;

use log::warn;
use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;

use crate::nodes_bin::{
    node::NodeType,
    node_error::NodeError,
    node_message::{ChildMessage, ParentMessage},
};

#[derive(Debug)]
pub struct NodeHandle {
    pub element: NodeType,
    pub name: String,
    pub id: String,
    pub children_names: Vec<String>,
    pub children_ids: Vec<String>,
    handles: Vec<NodeHandle>,
    tx: Sender<ChildMessage>, // This handle is held by a parent, so it can send child messages
    rx: Receiver<ParentMessage>, // The parent can receive messages from its child, so can listen to the handle for messages
}

impl Clone for NodeHandle {
    fn clone(&self) -> NodeHandle {
        Self {
            rx: self.rx.resubscribe(), // An rx cannot be cloned, but it can be created by subscribing to the transmitter
            tx: self.tx.clone(),
            element: self.element.clone(),
            name: self.name.clone(),
            id: self.id.clone(),
            handles: self.handles.clone(),
            children_names: self.children_names.clone(),
            children_ids: self.children_ids.clone(),
        }
    }
}

impl NodeHandle {
    pub fn new<T>(
        tx: Sender<ChildMessage>,
        rx: Receiver<ParentMessage>,
        element: NodeType,
        name: T,
        children_names: Vec<String>,
        children_ids: Vec<String>,
        handles: Vec<NodeHandle>,
    ) -> NodeHandle
    where
        T: Into<String>,
    {
        Self {
            tx,
            rx,
            element: element,
            name: name.into(),
            id: Uuid::new_v4().to_string(),
            handles,
            children_names,
            children_ids,
        }
    }

    pub(crate) async fn kill(&mut self) {
        if self.tx.receiver_count() > 0 {
            // It already exited if no receiver is alive
            if self.send(ChildMessage::Kill).is_err() {
                log::debug!(
                    "Send failed for ChildMessage::Kill - {:?} {:?} already exited",
                    self.element,
                    self.name
                );
                return;
            };
            loop {
                match self.listen().await {
                    Ok(ParentMessage::Killed) => {
                        log::debug!(
                            "Received ParentMessage::Killed from {:?} {}",
                            self.element,
                            self.name
                        );
                        return;
                    }
                    Ok(_) => {} // Some other message that can be discarded
                    Err(NodeError::TokioBroadcastRecvError(_)) => {
                        log::debug!(
                            "Error while listening to child - {:?} {} already exited",
                            self.element,
                            self.name
                        );
                        return;
                    }
                    Err(e) => log::debug!("Node error received {e:?}"),
                };
            }
        } else {
            log::debug!("{:?} {:?} already exited", self.element, self.name);
        }
    }

    pub(crate) fn take_handles(&mut self) -> Vec<NodeHandle> {
        let mut handles = std::mem::take(&mut self.handles);
        handles.push(self.clone());
        handles
    }

    pub(crate) fn send(&self, msg: ChildMessage) -> Result<(), NodeError> {
        self.tx.send(msg)?;
        Ok(())
    }

    pub(crate) async fn listen(&mut self) -> Result<ParentMessage, NodeError> {
        NodeHandle::_listen(&mut self.rx).await
    }

    async fn _listen(rx: &mut Receiver<ParentMessage>) -> Result<ParentMessage, NodeError> {
        Ok(rx.recv().await?)
    }

    pub(crate) async fn stop(&mut self) {
        if self.tx.receiver_count() > 0 {
            if let Err(err) = self.send(ChildMessage::Stop) {
                warn!("{:?} {:?} has error {:?}", self.element, self.name, err)
            }
            if let Err(err) = self.listen().await {
                warn!("{:?} {:?} has error {:?}", self.element, self.name, err)
            }
        } else {
            log::debug!("{:?} {:?} already exited", self.element, self.name);
        }
    }
}

impl PartialEq for NodeHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for NodeHandle {}

impl Hash for NodeHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
