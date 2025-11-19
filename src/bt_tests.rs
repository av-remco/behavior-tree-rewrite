#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use actify::Handle;
    use log::warn;
    use tokio::sync::mpsc::Receiver;
    use crate::bt::handle::Status;
    use crate::{BehaviorTree, NodeError, NodeHandle};
    use std::collections::HashMap;
    use tokio::time::{Duration, sleep};

    use crate::bt::*;
    use crate::bt::{
        action::{Failure, Success},
        condition::{Condition},
        selector::{Sequence, Fallback},
    };
    use crate::bt::action::mocking::MockAction;
    use crate::logging::load_logger;
    use crate::Wait;
    use logtest::Logger;

    #[tokio::test]
async fn test_auto_failure() {
    let action1 = Failure::new();
    let mut bt = BehaviorTree::new_test(action1.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        action1,                  // action visited
    ]);
}

#[tokio::test]
async fn test_auto_success() {
    let action1 = Success::new();
    let mut bt = BehaviorTree::new_test(action1.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        action1,
    ]);
}

#[tokio::test]
async fn test_condition_true_stops_at_condition() {
    let cond = Condition::new("cond1", Handle::new(5), |x| x > 0);
    let mut bt = BehaviorTree::new_test(cond.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        cond,                    // condition entered → stops
    ]);
}

#[tokio::test]
async fn test_sequence_hits_first_action() {
    let a1 = Success::new();
    let a2 = Success::new();
    let seq = Sequence::new(vec![a1.clone(), a2.clone()]);

    let mut bt = BehaviorTree::new_test(seq.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        seq,                     // enter sequence
        a1,                      // first actionable child
    ]);
}

#[tokio::test]
async fn test_sequence_condition_stops_sequence() {
    let cond = Condition::new("cond_seq", Handle::new(0), |x| x > 0);
    let a2 = Success::new();

    let seq = Sequence::new(vec![cond.clone(), a2.clone()]);
    let mut bt = BehaviorTree::new_test(seq.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        seq,                     // enter sequence
        cond,                    // condition → STOP
    ]);
}

#[tokio::test]
async fn test_fallback_first_child_fails_second_is_action() {
    let fail1 = Failure::new();
    let succ = Success::new();

    let fb = Fallback::new(vec![fail1.clone(), succ.clone()]);
    let mut bt = BehaviorTree::new_test(fb.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        fb,                      // enter fallback
        fail1,                   // first child
        succ,                    // fallback tries next child
    ]);
}

#[tokio::test]
async fn test_fallback_condition_as_first_child() {
    let cond = Condition::new("cond_fb", Handle::new(0), |x| x > 0);
    let a2  = Success::new();

    let fb = Fallback::new(vec![cond.clone(), a2.clone()]);
    let mut bt = BehaviorTree::new_test(fb.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        fb,                      // enter fallback
        cond,                    // stops → fallback does NOT continue
    ]);
}

#[tokio::test]
async fn test_nested_sequence_and_fallback() {
    // Sequence:
    //   cond  → stops (no fallback entered)
    //   fallback(fail, act)
    //
    // Condition prevents entering fallback.

    let cond = Condition::new("cond_nested", Handle::new(1), |x| x > 0);

    let fail = Failure::new();
    let act = Success::new();
    let fb = Fallback::new(vec![fail.clone(), act.clone()]);

    let seq = Sequence::new(vec![cond.clone(), fb.clone()]);
    let mut bt = BehaviorTree::new_test(seq.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        seq,                     // enter sequence
        cond,                    // stops → fallback never visited
    ]);
}

#[tokio::test]
async fn test_fallback_sequence_condition_then_action() {
    // fallback:
    //   sequence(cond → action)
    //   action2
    //
    // Sequence will stop at condition → so fallback won't go deeper.

    let cond = Condition::new("cond1", Handle::new(3), |x| x > 0);

    let a1 = Success::new();
    let seq = Sequence::new(vec![cond.clone(), a1.clone()]);

    let a2 = Success::new();
    let fb = Fallback::new(vec![seq.clone(), a2.clone()]);

    let mut bt = BehaviorTree::new_test(fb.clone());

    let trace = bt.search_start().await;

    assert_eq!(trace, vec![
        fb,                      // enter fallback
        seq,                     // enter sequence
        cond,                    // stops sequence
    ]);
}

}