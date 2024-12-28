use async_openai::error::OpenAIError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for tool operations
#[derive(Error, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum ToolError {
    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Error type for node operations
#[derive(Error, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum NodeError {
    #[error("Node execution error: {0}")]
    Execution(String),

    #[error(transparent)]
    Tool(#[from] ToolError),

    #[error("Subgraph execution error: {0}")]
    SubgraphExecution(String),
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

    #[error("Graph execution error: {0}")]
    ExecutionError(String),

    // NodeError can bubble up automatically
    #[error(transparent)]
    Node(#[from] NodeError),

    #[error("LLM error: {0}")]
    LLMError(String),

    // Catch-all for other errors like anyhow
    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for GraphError {
    fn from(err: anyhow::Error) -> Self {
        GraphError::Other(err.to_string())
    }
}

impl From<OpenAIError> for GraphError {
    fn from(err: OpenAIError) -> Self {
        GraphError::LLMError(err.to_string())
    }
}
