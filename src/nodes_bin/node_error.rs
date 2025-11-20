use actify::CacheRecvNewestError;
use thiserror::Error;
use tokio::sync::broadcast::error::SendError;

#[derive(Error, Debug, PartialEq, Clone)]
pub enum NodeError {
    #[error("The node is killed")]
    KillError,
    #[error("Poison error: {0}")]
    PoisonError(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Tokio broadcast send error: {0}")]
    TokioBroadcastSendError(String),
    #[error("Tokio broadcast receiver error")]
    TokioBroadcastRecvError(#[from] tokio::sync::broadcast::error::RecvError),
    #[error("Cache Error")]
    CacheError(#[from] CacheRecvNewestError),
}

impl<T> From<SendError<T>> for NodeError {
    fn from(err: SendError<T>) -> NodeError {
        NodeError::TokioBroadcastSendError(err.to_string())
    }
}

impl From<anyhow::Error> for NodeError {
    fn from(err: anyhow::Error) -> NodeError {
        NodeError::ExecutionError(err.to_string())
    }
}