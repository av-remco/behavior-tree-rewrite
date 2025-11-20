use std::hash::Hash;

use anyhow::Result;
use simple_xml_builder::XMLElement;

use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;

use crate::nodes_bin::{
    node::NodeType,
    node_error::NodeError,
    node_message::{ChildMessage, FutResponse, ParentMessage},
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

    pub fn take_handles(&mut self) -> Vec<NodeHandle> {
        let mut handles = std::mem::take(&mut self.handles);
        handles.push(self.clone());
        handles
    }

    pub fn send(&self, msg: ChildMessage) -> Result<(), NodeError> {
        self.tx.send(msg)?;
        Ok(())
    }

    pub fn get_rx(&mut self) -> Receiver<ParentMessage> {
        self.rx.resubscribe()
    }

    // Consuming and returning the receiver allows stacking it in a future vector
    pub async fn run_listen(
        mut rx: Receiver<ParentMessage>,
        child_index: usize,
    ) -> Result<FutResponse, NodeError> {
        let msg = NodeHandle::_listen(&mut rx).await?;
        Ok(FutResponse::Child(child_index, msg, rx)) // The rx is returned to ensure the channel is fully read
    }

    pub async fn listen(&mut self) -> Result<ParentMessage, NodeError> {
        NodeHandle::_listen(&mut self.rx).await
    }

    async fn _listen(rx: &mut Receiver<ParentMessage>) -> Result<ParentMessage, NodeError> {
        Ok(rx.recv().await?)
    }

    pub fn get_xml(&self) -> XMLElement {
        let element = match self.element {
            NodeType::Condition => String::from("Decorator"), // Groot sees any condition as decorator
            NodeType::Action => String::from("Action"),
            NodeType::Sequence => String::from("Sequence"),
            NodeType::Fallback => String::from("Fallback"),
        };

        let mut element = XMLElement::new(element);
        element.add_attribute("name", &self.name);
        element
    }

    pub fn get_json(&self) -> serde_json::value::Value {
        if !self.children_names.is_empty() {
            serde_json::json!({
                "id": self.id.clone(),
                "name": self.name.clone(),
                "type": self.element.clone(),
                "children": self.children_ids.clone()})
        } else {
            serde_json::json!({
                "id": self.id.clone(),
                "name": self.name.clone(),
                "type": self.element.clone()})
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
