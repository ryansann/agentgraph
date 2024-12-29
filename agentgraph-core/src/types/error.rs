use async_openai::error::OpenAIError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for tool operations
#[derive(Error, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum ToolError {
    #[error("Schema: {0}")]
    Schema(String),

    #[error("Execution: {0}")]
    Execution(String),

    #[error("Serialization: {0}")]
    Serialization(String),
}

/// Error type for node operations
#[derive(Error, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum NodeError {
    #[error("Node execution: {0}")]
    Execution(String),

    #[error(transparent)]
    Tool(#[from] ToolError),

    #[error("Model: {0}")]
    ModelError(String),

    #[error("Subgraph execution: {0}")]
    SubgraphExecution(String),

    #[error("Other: {0}")]
    Other(String),
}

impl From<anyhow::Error> for NodeError {
    fn from(err: anyhow::Error) -> Self {
        NodeError::Other(err.to_string())
    }
}

impl From<OpenAIError> for NodeError {
    fn from(err: OpenAIError) -> Self {
        NodeError::ModelError(err.to_string())
    }
}

/// Error type for overall graph operations
#[derive(Error, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum GraphError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Invalid transition: {0}")]
    InvalidTransition(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Execution: {0}")]
    ExecutionError(String),

    // NodeError can bubble up automatically
    #[error(transparent)]
    Node(#[from] NodeError),

    #[error("Model: {0}")]
    ModelError(String),

    // Catch-all for other errors like anyhow
    #[error("Other: {0}")]
    Other(String),
}

impl From<anyhow::Error> for GraphError {
    fn from(err: anyhow::Error) -> Self {
        GraphError::Other(err.to_string())
    }
}

impl From<OpenAIError> for GraphError {
    fn from(err: OpenAIError) -> Self {
        GraphError::ModelError(err.to_string())
    }
}
