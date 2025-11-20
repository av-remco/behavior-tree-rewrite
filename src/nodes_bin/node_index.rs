use crate::nodes_bin::node_handle::NodeHandle;

pub struct NodeIndex {
    pub handles: Vec<NodeHandle>,
}

impl NodeIndex {
    pub fn new(handles: Vec<NodeHandle>) -> Self {
        Self { handles }
    }

    pub fn get(&self, id: &str) -> Option<NodeHandle> {
        self.handles
            .iter()
            .find(|x| x.id == id)
            .cloned()
    }
}
