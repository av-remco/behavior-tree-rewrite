use std::collections::{HashMap, HashSet, VecDeque};

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
    handles: Vec<NodeHandle>,
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
        let handles = root_node.take_handles();
        Self {
            name,
            root_node,
            handles,
        }
    }

    pub(crate) fn search_start(&self) -> Vec<NodeHandle> {
        self.rec_search_down(self.root_node.clone(), vec![])
    }

    fn rec_search_down(&self, mut node: NodeHandle, mut trace: Vec<NodeHandle>) -> Vec<NodeHandle> {
        println!("Searching down from {:?}", node.name);
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
                if let Some(child) = self.get_node_handle_by_id(id) {
                    trace.push(node.clone());
                    self.rec_search_down(child, trace)
                } else {
                    panic!("Did not find Handle for child");
                }
            }
        }
    }

    pub(crate) fn search_next(&self, trace: Vec<NodeHandle>, result: &Status) -> Vec<NodeHandle> {
        self.rec_search_up(trace, result, None)
    }

    fn rec_search_up(&self, mut trace: Vec<NodeHandle>, result: &Status, id: Option<String>) -> Vec<NodeHandle> {
        match trace.pop() {
            Some(node) => {
                println!("Searching up for {:?}", node.name);
                match (node.element, result) {
                    (NodeType::Fallback, Status::Failure) | (NodeType::Sequence, Status::Success) => {
                        // If previous node was not the last child, select next child and search down
                        if let Some(id) = id {
                            let next_id = node.children_ids.iter()
                                .position(|x| *x == *id)
                                .and_then(|i| node.children_ids.get(i + 1))
                                .cloned();

                            if let Some(next_id) = next_id {
                                // Next child found, search down
                                if let Some(child) = self.get_node_handle_by_id(next_id) {
                                    trace.push(node.clone());
                                    self.rec_search_down(child, trace)
                                } else {
                                    panic!("Did not find Handle for child");
                                }
                            } else {
                                // No next child, i.e. previous was last child
                                self.rec_search_up(trace, result, Some(node.id))
                            }
                        } else {
                            // No previous node
                            self.rec_search_up(trace, result, Some(node.id))
                        }
                    },
                    (_,_) => self.rec_search_up(trace, result, Some(node.id))
                }
            },
            None => return trace,
        }
    }

    fn get_node_handle_by_id(&self, id: String) -> Option<NodeHandle> {
        let handle = self.handles
            .iter()
            .find(|x| x.id == id)
            .cloned()
            .expect("A handle was not present in the node handles!");

        Some(handle)
    }
}

type BehaviorTreeMap = HashMap<(NodeHandle, Status), Option<NodeHandle>>;

pub async fn convert_bt(bt: &mut BehaviorTree) -> BehaviorTreeMap {
    let mut map = HashMap::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // 1. Start: search_start() returns Vec<NodeHandle>
    let mut start_vec = bt.search_start();
    if let Some(start) = start_vec.last().cloned() {
        queue.push_back(start_vec.clone());
        visited.insert(start.clone());

        while let Some(current) = queue.pop_front() {
            // Try SUCCESS
            let succ_vec = bt.search_next(current.clone(), &Status::Success);
            let succ_next = succ_vec.last().cloned(); // Option<NodeHandle>
            map.insert((current.last().cloned().unwrap(), Status::Success), succ_next.clone());

            if let Some(next) = succ_next.clone() {
                if !visited.contains(&next) {
                    visited.insert(next.clone());
                    queue.push_back(succ_vec.clone());
                }
            }

            // Try FAILURE
            let fail_vec = bt.search_next(current.clone(), &Status::Failure);
            let fail_next = fail_vec.last().cloned();
            map.insert((current.last().cloned().unwrap(), Status::Failure), fail_next.clone());

            if let Some(next) = fail_next.clone() {
                if !visited.contains(&next) {
                    visited.insert(next.clone());
                    queue.push_back(fail_vec.clone());
                }
            }
        }
    }

    map
}
