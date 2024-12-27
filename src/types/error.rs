use thiserror::Error;

/// Core error types for the graphite library
#[derive(Error, Debug)]
pub enum GraphError {
    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("invalid transition: {0}")]
    InvalidTransition(String),

    #[error("graph execution error: {0}")]
    ExecutionError(String),

    #[error("tool execution error: {0}")]
    ToolError(String),

    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
