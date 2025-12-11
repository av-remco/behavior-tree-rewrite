use std::pin::Pin;

use crate::nodes_bin::{node_error::NodeError, node_map::NodeIdToProcessHandleMap, node_message::{ChildMessage, FutResult}, process_handle::ProcessHandle};

// Shorten Future type
pub type FutureVec<'a> = Vec<Pin<Box<dyn Future<Output = FutResult> + Send + 'a>>>;

pub(super) struct ProcessComms {
    map: NodeIdToProcessHandleMap
}

impl ProcessComms {
    pub fn new(map: NodeIdToProcessHandleMap) -> ProcessComms {
        Self { map }
    }

    pub async fn send(&mut self, id: String, msg: ChildMessage) -> Result<(), NodeError>{
        if let Some(handle) = self.map.get_mut(&id) {
            return handle.send(msg).await
        }
        Err(NodeError::ExecutionError("No process found!".to_string()))
    }

    pub fn get_handle(&mut self, id: String) -> Result<&mut ProcessHandle, NodeError>{
        if let Some(handle) = self.map.get_mut(&id) {
            return Ok(handle);
        }
        Err(NodeError::ExecutionError("No process found!".to_string()))
    }
}