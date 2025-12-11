use log::warn;

use crate::{BT, bt::Ready, nodes_bin::{node::Node, node_status::Status}};

pub(crate) fn search_start(tree: &BT<Ready>) -> Vec<Node> {
    search_down(tree.root.clone(), vec![])
}

fn search_down(node: Node, mut trace: Vec<Node>) -> Vec<Node> {
    match &node {
        Node::Action(_) | Node::Condition(_) => {
            trace.push(node.clone());
            trace
        },
        Node::Fallback(children) | Node::Sequence(children) => {
            if let Some(child) = children.first() {
                trace.push(node.clone());
                search_down( child.clone(), trace)
            } else {
                warn!("Found empty selector!");
                vec![]
            }
        }
    }
}

pub(crate) fn search_next(trace: Vec<Node>, result: &Status) -> Vec<Node> {
    search_up(trace, result, None)
}

fn search_up(mut trace: Vec<Node>, result: &Status, previous_node: Option<Node>) -> Vec<Node> {
    // If trace = [], we have reached the root
    let Some(node) = trace.pop() else {
        return trace;
    };

    match (&node, result) {
        // If previous node was not the last child of a selector, select next child and search down
        (Node::Fallback(children), Status::Failure) | 
        (Node::Sequence(children), Status::Success) => {
            if let Some(previous_node) = previous_node {
                if let Some(next_child) = children.iter()
                    .position(|node| *node == previous_node)
                    .and_then(|i| children.get(i + 1))
                    .cloned()
                {
                    trace.push(node.clone());
                    return search_down(next_child, trace);
                }
            }
        },
        (Node::Action(_) | Node::Condition(_) | Node::Sequence(_) | Node::Fallback(_),_) => ()
    }
    search_up(trace, result, Some(node))
}