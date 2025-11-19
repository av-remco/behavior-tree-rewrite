use crate::NodeHandle;

pub mod handle;
pub mod nodes;
pub mod action;
pub mod condition;
pub mod selector;

const CHANNEL_SIZE: usize = 20;

pub struct BehaviorTree {
    pub name: String,
    root_node: NodeHandle,
}

impl BehaviorTree {
    pub fn new<S: Into<String>>(root_node: NodeHandle, name: S) -> Self {
        BehaviorTree::_new(root_node, name.into())
    }

    #[cfg(test)]
    pub fn new_test(root_node: NodeHandle) -> Self {
        BehaviorTree::_new(root_node, "test-tree".to_string())
    }

    fn _new(mut root_node: NodeHandle, name: String) -> Self {
        Self {
            name,
            root_node,
        }
    }

    pub(crate) async fn search_start(&self) -> Vec<NodeHandle> {
        vec![]
    }
}