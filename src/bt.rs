use crate::{NodeHandle, NodeType};

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
        self.rec_search_start(self.root_node.clone(), vec![])
    }

    fn rec_search_start(&self, mut node: NodeHandle, mut trace: Vec<NodeHandle>) -> Vec<NodeHandle> {
        match node.element {
            NodeType::Action | NodeType::Condition => {
                trace.push(node.clone());
                trace
            },
            NodeType::Fallback | NodeType::Sequence => {
                println!("{:?}", node.name);
                println!("{:?}", node.id);
                if let Some(child) = node.get_first_child() {
                    trace.push(node.clone());
                    self.rec_search_start(child, trace)
                } else {
                    panic!("Found a selector without a child");
                }
            }
        }
    }
}