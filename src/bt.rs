use crate::nodes_bin::{node_handle::NodeHandle, node_index::NodeIndex};

pub(crate) const CHANNEL_SIZE: usize = 20;

pub struct BehaviorTree {
    pub name: String,
    pub root: NodeHandle,
    pub(crate) index: NodeIndex,
}

impl BehaviorTree {
    pub fn new<S: Into<String>>(root_node: NodeHandle, name: S) -> Self {
        BehaviorTree::_new(root_node, name.into())
    }

    #[cfg(test)]
    pub fn new_test(root_node: NodeHandle) -> Self {
        BehaviorTree::_new(root_node, "test-tree".to_string())
    }

    fn _new(mut root: NodeHandle, name: String) -> Self {
        let handles = root.take_handles();
        Self {
            name,
            root,
            index: NodeIndex::new(handles),
        }
    }
}