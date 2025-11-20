use crate::{BT, bt::{BuildState, Built, Converting, Executing}, nodes_bin::{node::NodeType, node_handle::NodeHandle, node_status::Status}};

trait TraversalState {}
impl TraversalState for Converting {}
impl TraversalState for Built {}
impl TraversalState for Executing {}

pub(crate) fn search_start<T: TraversalState + BuildState>(tree: &BT<T>) -> Vec<NodeHandle> {
    rec_search_down(tree, tree.root.clone(), vec![])
}

fn rec_search_down<T: TraversalState + BuildState>(tree: &BT<T>, node: NodeHandle, mut trace: Vec<NodeHandle>) -> Vec<NodeHandle> {
    match node.element {
        NodeType::Action | NodeType::Condition => {
            trace.push(node.clone());
            trace
        },
        NodeType::Fallback | NodeType::Sequence => {
            let id = match node.children_ids.first() {
                Some(id) => id,
                None => panic!("Found a selector without a child"),
            };
            if let Some(child) = tree.index.get(id) {
                trace.push(node.clone());
                rec_search_down(tree, child, trace)
            } else {
                panic!("Did not find Handle for child");
            }
        }
    }
}

pub(crate) fn search_next<T: TraversalState + BuildState>(tree: &BT<T>, trace: Vec<NodeHandle>, result: &Status) -> Vec<NodeHandle> {
    rec_search_up(tree, trace, result, None)
}

fn rec_search_up<T: TraversalState + BuildState>(tree: &BT<T>, mut trace: Vec<NodeHandle>, result: &Status, previous_node_id: Option<String>) -> Vec<NodeHandle> {
    // If trace = [], we have reached the root
    let Some(node) = trace.pop() else {
        return trace;
    };

    match (node.element, result) {
        // If previous node was not the last child, select next child and search down
        (NodeType::Fallback, Status::Failure) | 
        (NodeType::Sequence, Status::Success) => {
            if let Some(child) = previous_node_id
                .as_ref()
                .and_then(|id| next_sibling(&node, id))
                .and_then(|next_id| tree.index.get(&next_id))
            {
                trace.push(node.clone());
                return rec_search_down(tree, child, trace);
            }
        },
        (_,_) => ()
    }
    
    rec_search_up(tree, trace, result, Some(node.id))
}

fn next_sibling(node: &NodeHandle, child_id: &str) -> Option<String> {
    node.children_ids.iter()
        .position(|x| *x == child_id)
        .and_then(|i| node.children_ids.get(i + 1))
        .cloned()
}