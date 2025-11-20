use std::collections::{HashMap, HashSet, VecDeque};

use crate::{BehaviorTree, NodeHandle, bt::{handle::Status, traversal::{search_next, search_start}}};

type BehaviorTreeMap = HashMap<(NodeHandle, Status), Option<NodeHandle>>;

pub(crate) fn convert_bt(bt: &mut BehaviorTree) -> BehaviorTreeMap {
    let mut map = HashMap::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // 1. Start: search_start() returns Vec<NodeHandle>
    let start_vec = search_start(&bt);
    if let Some(start) = start_vec.last().cloned() {
        queue.push_back(start_vec.clone());
        visited.insert(start.clone());

        while let Some(current) = queue.pop_front() {
            // Try SUCCESS
            let succ_vec = search_next(&bt, current.clone(), &Status::Success);
            let succ_next = succ_vec.last().cloned(); // Option<NodeHandle>
            if let Some(current_node) = current.last().cloned() {
                map.insert((current_node, Status::Success), succ_next.clone());
            }

            if let Some(next) = succ_next.clone() {
                if !visited.contains(&next) {
                    visited.insert(next.clone());
                    queue.push_back(succ_vec.clone());
                }
            }

            // Try FAILURE
            let fail_vec = search_next(&bt, current.clone(), &Status::Failure);
            let fail_next = fail_vec.last().cloned();
            if let Some(current_node) = current.last().cloned() {
                map.insert((current_node, Status::Failure), fail_next.clone());
            }

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