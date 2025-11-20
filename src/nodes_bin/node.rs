pub trait Node: Sync + Send {
    async fn serve(self);
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum NodeType {
    Action,
    Condition,
    Fallback,
    Sequence,
}