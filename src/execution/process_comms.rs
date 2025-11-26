use crate::nodes_bin::{node_error::NodeError, node_map::NodeHandleMap, node_message::ChildMessage, process_handle::ProcessHandle};

pub(super) struct ProcessComms {
    map: NodeHandleMap
}

impl ProcessComms {
    pub fn new(map: NodeHandleMap) -> ProcessComms {
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