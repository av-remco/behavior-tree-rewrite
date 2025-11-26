use anyhow::Result;

use tokio::sync::broadcast::{Receiver, Sender};

use crate::nodes_bin::{
    node_error::NodeError,
    node_message::{ChildMessage, ParentMessage}, node_status::Status,
};

#[derive(Debug)]
pub struct ProcessHandle {
    tx: Sender<ChildMessage>, // This handle is held by a parent, so it can send child messages
    rx: Receiver<ParentMessage>, // The parent can receive messages from its child, so can listen to the handle for messages
    name: String,
}

impl Clone for ProcessHandle {
    fn clone(&self) -> ProcessHandle {
        Self {
            rx: self.rx.resubscribe(), // An rx cannot be cloned, but it can be created by subscribing to the transmitter
            tx: self.tx.clone(),
            name: self.name.clone(),
        }
    }
}

impl ProcessHandle {
    pub(crate) fn new<T>(
        tx: Sender<ChildMessage>,
        rx: Receiver<ParentMessage>,
        name: T,
    ) -> ProcessHandle
    where
        T: Into<String>,
    {
        Self {
            tx,
            rx,
            name: name.into(),
        }
    }

    pub(crate) async fn send(&mut self, msg: ChildMessage) -> Result<(), NodeError> {
        // Fire-and-forget for normal messages
        let requires_reply = matches!(msg, ChildMessage::Kill | ChildMessage::Stop);

        // If no child alive, treat as already exited
        if self.tx.receiver_count() == 0 {
            log::debug!("{:?} already exited (no receiver)", self.name);
            return Ok(());
        }

        if let Err(err) = self.tx.send(msg) {
            log::debug!(
                "Send failed for {:?} to {} - already exited?",
                err, self.name
            );
            return Err(err.into());
        }

        if requires_reply {
            loop {
                match self.listen().await {
                    Ok(ParentMessage::Killed) => {
                        log::debug!("Received ParentMessage::Killed from {}", self.name);
                        return Ok(());
                    }
                    Ok(ParentMessage::Status(Status::Idle)) => {
                        log::debug!("Received Status::Idle from {}", self.name);
                        return Ok(());
                    }
                    Ok(other) => {
                        // Ignore unrelated messages (same as before)
                        log::trace!("Discarding unrelated parent message: {:?}", other);
                    }
                    Err(NodeError::TokioBroadcastRecvError(_)) => {
                        log::debug!(
                            "Error while listening to child - {} already exited",
                            self.name
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        log::debug!("Node error received while waiting: {:?}", e);
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    pub(crate) async fn listen(&mut self) -> Result<ParentMessage, NodeError> {
        ProcessHandle::_listen(&mut self.rx).await
    }

    async fn _listen(
        rx: &mut Receiver<ParentMessage>,
    ) -> Result<ParentMessage, NodeError> {
        Ok(rx.recv().await?)
    }
}