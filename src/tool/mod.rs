// src/tool/mod.rs
pub use agentgraph_macros::tool;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::error::Error;
use async_trait::async_trait;

// Re-export key types and traits
pub use async_openai::types::{
    ChatCompletionTool,
    ChatCompletionToolType,
    FunctionObject,
};

/// Error type for tool operations
#[derive(Debug)]
pub enum ToolError {
    /// Error during schema generation or validation
    Schema(String),
    /// Error during tool execution
    Execution(Box<dyn Error + Send + Sync>),
    /// Error during serialization/deserialization
    Serialization(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(msg) => write!(f, "Schema error: {}", msg),
            Self::Execution(err) => write!(f, "Execution error: {}", err),
            Self::Serialization(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl Error for ToolError {}

/// Trait for types that can be converted to JSON Schema
pub trait JsonSchema {
    /// Generate JSON Schema representation of the type
    fn schema() -> Value;
}

/// Trait that must be implemented by OpenAI tool functions
#[async_trait]
pub trait ToolFunction {
    /// The parameter type for the tool
    type Params: JsonSchema + DeserializeOwned;
    /// The response type for the tool
    type Response: JsonSchema + Serialize;

    /// Get the name of the tool
    fn name() -> &'static str;

    /// Get a description of what the tool does
    fn description() -> &'static str;

    /// Get the JSON Schema for the tool's parameters
    fn parameters_schema() -> Value {
        Self::Params::schema()
    }

    /// Get the JSON Schema for the tool's response
    fn response_schema() -> Value {
        Self::Response::schema()
    }

    /// Get the complete tool schema for OpenAI
    fn get_schema() -> ChatCompletionTool {
        ChatCompletionTool {
            r#type: ChatCompletionToolType::Function,
            function: FunctionObject {
                name: Self::name().to_string(),
                description: Some(Self::description().to_string()),
                parameters: Some(Self::parameters_schema()),
                strict: Some(true),
            },
        }
    }

    /// Execute the tool with the given parameters
    async fn execute(&self, params: Self::Params) -> Result<Self::Response, ToolError>;
}

// Helper function for implementations
pub(crate) fn to_tool_error<E: Error + Send + Sync + 'static>(err: E) -> ToolError {
    ToolError::Execution(Box::new(err))
}

