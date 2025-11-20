#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum NodeType {
    Action,
    Condition,
    Fallback,
    Sequence,
}