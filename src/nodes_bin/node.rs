pub trait NodeProcess: Sync + Send {
    async fn serve(self);
}

pub(crate) type NodeId = String;

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, Hash)]
pub enum Node {
    Action(NodeId),
    Condition(NodeId),
    Sequence(Vec<Node>),
    Fallback(Vec<Node>),
}

impl Node {
    pub fn get_id(&self) -> Option<String> {
        match self {
            Node::Action(id) => Some(id.to_string()),
            Node::Condition(id) => Some(id.to_string()),
            Node::Sequence(_) => None,
            Node::Fallback(_) => None,
        }
    }
}