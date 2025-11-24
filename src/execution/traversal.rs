use crate::{BT, bt::Ready, nodes_bin::{node::NodeType, node_handle::NodeHandle, node_status::Status}};

pub(crate) fn search_start(tree: &BT<Ready>) -> Vec<NodeHandle> {
    let Some(root) = tree.root.clone() else {
        return vec![];
    };
    rec_search_down(tree, root, vec![])
}

fn rec_search_down(tree: &BT<Ready>, node: NodeHandle, mut trace: Vec<NodeHandle>) -> Vec<NodeHandle> {
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
            if let Some(child) = tree.node_index.get(id) {
                trace.push(node.clone());
                rec_search_down(tree, child, trace)
            } else {
                panic!("Did not find Handle for child");
            }
        }
    }
}

pub(crate) fn search_next(tree: &BT<Ready>, trace: Vec<NodeHandle>, result: &Status) -> Vec<NodeHandle> {
    rec_search_up(tree, trace, result, None)
}

fn rec_search_up(tree: &BT<Ready>, mut trace: Vec<NodeHandle>, result: &Status, previous_node_id: Option<String>) -> Vec<NodeHandle> {
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
                .and_then(|next_id| tree.node_index.get(&next_id))
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