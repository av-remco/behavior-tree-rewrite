use crate::{BT, bt::Ready, nodes_bin::{node::Node, node_status::Status}};

pub(crate) fn search_start(tree: &BT<Ready>) -> Vec<Node> {
    let Some(root) = tree.root.clone() else {
        return vec![];
    };
    rec_search_down(root, vec![])
}

fn rec_search_down(node: Node, mut trace: Vec<Node>) -> Vec<Node> {
    match &node {
        Node::Action(_) | Node::Condition(_) => {
            trace.push(node.clone());
            trace
        },
        Node::Fallback(children) | Node::Sequence(children) => {
            if let Some(child) = children.first() {
                trace.push(node.clone());
                rec_search_down( child.clone(), trace)
            } else {
                panic!("Found a selector without a child");
            }
        }
    }
}

pub(crate) fn search_next(trace: Vec<Node>, result: &Status) -> Vec<Node> {
    rec_search_up(trace, result, None)
}

fn rec_search_up(mut trace: Vec<Node>, result: &Status, previous_node: Option<Node>) -> Vec<Node> {
    // If trace = [], we have reached the root
    let Some(node) = trace.pop() else {
        return trace;
    };

    match (&node, result) {
        // If previous node was not the last child, select next child and search down
        (Node::Fallback(children), Status::Failure) | 
        (Node::Sequence(children), Status::Success) => {
            if let Some(previous_node) = previous_node {
                if let Some(next_child) = children.iter()
                    .position(|node| *node == previous_node)
                    .and_then(|i| children.get(i + 1))
                    .cloned()
                {
                    trace.push(node.clone());
                    return rec_search_down(next_child, trace);
                }
            }
        },
        (_,_) => ()
    }
    rec_search_up(trace, result, Some(node))
}