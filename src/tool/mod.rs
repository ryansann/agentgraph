use async_trait::async_trait;
use schemars as sm; // rename for convenience
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sm::JsonSchema as SchemarsJsonSchema;
use std::error::Error;

// Re-export key types and traits
pub use async_openai::types::{ChatCompletionTool, ChatCompletionToolType, FunctionObject};

/// Error type for tool operations
#[derive(Debug, Serialize)]
pub enum ToolError {
    Schema(String),
    Execution(String),
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

/// Helper function for implementations
pub(crate) fn to_tool_error<E: Error + Send + Sync + 'static>(err: E) -> ToolError {
    ToolError::Execution(err.to_string())
}

/// Our existing trait
pub trait JsonSchema {
    /// Generate JSON Schema representation
    fn schema() -> Value;
}

/// Blanket impl: for any type that implements `schemars::JsonSchema`,
/// our trait just calls schemars::schema_for! to get the schema,
/// then converts it to `serde_json::Value`.
impl<T> JsonSchema for T
where
    T: SchemarsJsonSchema,
{
    fn schema() -> Value {
        // schemars::schema_for!(T) returns a `Schema`.
        // We convert it to JSON via `serde_json::to_value`.
        let schema_obj = sm::schema_for!(T).schema;
        serde_json::to_value(&schema_obj).unwrap_or_else(
            |_| serde_json::json!({ "type": "object", "description": "error generating schema" }),
        )
    }
}

/// Trait that must be implemented by OpenAI tool functions
#[async_trait]
pub trait ToolFunction {
    /// The parameter type for the tool
    ///
    /// Must implement our `JsonSchema` trait + `DeserializeOwned`.
    /// Because of the blanket impl, it also needs `schemars::JsonSchema`.
    type Params: JsonSchema + DeserializeOwned;

    /// The response type for the tool
    type Response: Serialize;

    fn name() -> &'static str;
    fn description() -> &'static str;

    fn parameters_schema() -> Value {
        // By default: Self::Params::schema()
        Self::Params::schema()
    }

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

    async fn execute(&self, params: Self::Params) -> Result<Self::Response, ToolError>;
}
