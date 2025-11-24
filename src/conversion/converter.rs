use std::collections::{HashMap, HashSet, VecDeque};

use crate::{BT, bt::Ready, execution::traversal::{search_next, search_start}, nodes_bin::{node_handle::NodeHandle, node_status::Status}};


pub(crate) type BehaviorTreeMap = HashMap<(NodeHandle, Status), Option<NodeHandle>>;

pub(crate) fn convert_bt(bt: &BT<Ready>) -> BehaviorTreeMap {
    let mut map = HashMap::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    let start_vec = search_start(&bt);
    let Some(start) = start_vec.last().cloned() else { return map };

    queue.push_back(start_vec.clone());
    visited.insert(start.clone());

    while let Some(current) = queue.pop_front() {
        let Some(current_node) = current.last() else { continue };

        for &status in &[Status::Success, Status::Failure] {
            let next_vec = search_next(&bt, current.clone(), &status);
            let next_node = next_vec.last().cloned();

            // Insert into map
            map.insert((current_node.clone(), status), next_node.clone());

            // Enqueue trace if not node not already visited
            if let Some(next) = next_node {
                if visited.insert(next.clone()) {
                    queue.push_back(next_vec);
                }
            }
        }
    }

    map
}