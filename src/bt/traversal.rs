use crate::{BehaviorTree, NodeHandle, NodeType, bt::handle::Status};

pub(crate) fn search_start(tree: &BehaviorTree) -> Vec<NodeHandle> {
    rec_search_down(tree, tree.root.clone(), vec![])
}

fn rec_search_down(tree: &BehaviorTree, node: NodeHandle, mut trace: Vec<NodeHandle>) -> Vec<NodeHandle> {
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
            if let Some(child) = tree.get_node_handle_by_id(id) {
                trace.push(node.clone());
                rec_search_down(tree, child, trace)
            } else {
                panic!("Did not find Handle for child");
            }
        }
    }
}

pub(crate) fn search_next(tree: &BehaviorTree, trace: Vec<NodeHandle>, result: &Status) -> Vec<NodeHandle> {
    rec_search_up(tree, trace, result, None)
}

fn rec_search_up(tree: &BehaviorTree, mut trace: Vec<NodeHandle>, result: &Status, id: Option<String>) -> Vec<NodeHandle> {
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
                            if let Some(child) = tree.get_node_handle_by_id(next_id) {
                                trace.push(node.clone());
                                rec_search_down(tree, child, trace)
                            } else {
                                panic!("Did not find Handle for child");
                            }
                        } else {
                            // No next child, i.e. previous was last child
                            rec_search_up(tree, trace, result, Some(node.id))
                        }
                    } else {
                        // No previous node
                        rec_search_up(tree, trace, result, Some(node.id))
                    }
                },
                (_,_) => rec_search_up(tree, trace, result, Some(node.id))
            }
        },
        None => return trace,
    }
}