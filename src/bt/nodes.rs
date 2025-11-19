#[derive(Debug, Clone, serde::Serialize)]
pub enum NodeType {
    Action,
    Condition,
    Fallback,
    Sequence,
}