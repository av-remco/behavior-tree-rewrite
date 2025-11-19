use crate::{NodeHandle, NodeType, bt::handle::Status};

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

    pub(crate) fn search_start(&self) -> Vec<NodeHandle> {
        self.rec_search_down(self.root_node.clone(), vec![])
    }

    fn rec_search_down(&self, mut node: NodeHandle, mut trace: Vec<NodeHandle>) -> Vec<NodeHandle> {
        match node.element {
            NodeType::Action | NodeType::Condition => {
                trace.push(node.clone());
                trace
            },
            NodeType::Fallback | NodeType::Sequence => {
                let id = match node.children_ids.first().cloned() {
                    Some(id) => id,
                    None => panic!("Found a selector without a child"),
                };
                if let Some(child) = node.get_child_handle_by_id(id) {
                    trace.push(node.clone());
                    self.rec_search_down(child, trace)
                } else {
                    panic!("Did not find Handle for child");
                }
            }
        }
    }

    pub(crate) fn search_next(&self, trace: Vec<NodeHandle>, result: Status) -> Vec<NodeHandle> {
        self.rec_search_up(trace, result)
    }

    fn rec_search_up(&self, mut trace: Vec<NodeHandle>, result: Status) -> Vec<NodeHandle> {
        match trace.pop() {
            Some(node) => {
                match (node.element, result) {
                    (NodeType::Fallback, Status::Failure) => { /* ... */ },
                    (NodeType::Sequence, Status::Success) => { /* ... */ },
                    (_,_) => self.rec_search_up(trace, result)
                }
            },
            None => return trace,
        }
    }
}