use crate::nodes_bin::process_handle::ProcessHandle;

// Maps node.id to its ProcessHandle
pub type NodeIdToProcessHandleMap = std::collections::HashMap<String, ProcessHandle>;