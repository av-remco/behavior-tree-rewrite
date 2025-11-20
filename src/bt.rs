use crate::NodeHandle;

pub mod handle;
pub mod nodes;
pub mod action;
pub mod condition;
pub mod selector;
pub mod traversal;
pub mod converter;

const CHANNEL_SIZE: usize = 20;

pub struct BehaviorTree {
    pub name: String,
    pub root: NodeHandle,
    pub(crate) handles: Vec<NodeHandle>,
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
            handles,
        }
    }

    pub(crate) fn get_node_handle_by_id(&self, id: String) -> Option<NodeHandle> {
        let handle = self.handles
            .iter()
            .find(|x| x.id == id)
            .cloned()
            .expect("A handle was not present in the node handles!");

        Some(handle)
    }
}