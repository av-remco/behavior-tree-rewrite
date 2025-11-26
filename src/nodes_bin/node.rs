pub trait NodeProcess: Sync + Send {
    async fn serve(self);
}

pub(crate) type ActionId = String;
pub(crate) type ConditionId = String;

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub enum Node {
    Action(ActionId),
    Condition(ConditionId),
    Sequence(Vec<Node>),
    Fallback(Vec<Node>),
}