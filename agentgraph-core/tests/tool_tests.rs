use agentgraph_core::prelude::*;
use agentgraph_macros::tool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AddParams {
    x: i32,
    y: i32,
}

impl JsonSchema for AddParams {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            },
            "required": ["x", "y"]
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AddResponse {
    sum: i32,
}

impl JsonSchema for AddResponse {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "sum": {"type": "integer"}
            },
            "required": ["sum"]
        })
    }
}

#[derive(Clone)]
struct Add;

#[tool("Adds two numbers together")]
async fn add(_tool: &Add, params: AddParams) -> std::result::Result<AddResponse, ToolError> {
    Ok(AddResponse {
        sum: params.x + params.y,
    })
}

#[tokio::test]
async fn test_tool_execution() {
    let tool = Add;
    let params = AddParams { x: 5, y: 3 };

    let result = ToolFunction::execute(&tool, params).await.unwrap();
    assert_eq!(result.sum, 8);
}

#[test]
fn test_tool_schema() {
    let schema = <Add as ToolFunction>::get_schema();

    // Check function metadata
    assert_eq!(schema.function.name, "add");
    assert_eq!(
        schema.function.description.unwrap(),
        "Adds two numbers together"
    );

    // Check parameters schema
    let params = schema.function.parameters.unwrap();
    assert_eq!(
        params,
        serde_json::json!({
            "type": "object",
            "properties": {
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            },
            "required": ["x", "y"]
        })
    );
}

// Test optional parameters
#[derive(Debug, Serialize, Deserialize)]
struct OptionalParams {
    required: String,
    optional: Option<i32>,
}

impl JsonSchema for OptionalParams {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "required": {"type": "string"},
                "optional": {
                    "type": "integer",
                    "required": false
                }
            },
            "required": ["required"]
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OptionalResponse {
    result: String,
}

impl JsonSchema for OptionalResponse {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "result": {"type": "string"}
            },
            "required": ["result"]
        })
    }
}

#[derive(Clone)]
struct HandleOptional;

#[tool("A tool with optional parameters")]
async fn handle_optional(
    _tool: &HandleOptional,
    params: OptionalParams,
) -> std::result::Result<OptionalResponse, ToolError> {
    let result = format!(
        "Required: {}, Optional: {}",
        params.required,
        params
            .optional
            .map_or("None".to_string(), |n| n.to_string())
    );
    Ok(OptionalResponse { result })
}

#[tokio::test]
async fn test_optional_parameters() {
    let tool = HandleOptional;

    // Test with all parameters
    let params = OptionalParams {
        required: "test".to_string(),
        optional: Some(42),
    };
    let result = ToolFunction::execute(&tool, params).await.unwrap();
    assert_eq!(result.result, "Required: test, Optional: 42");

    // Test with only required parameters
    let params = OptionalParams {
        required: "test".to_string(),
        optional: None,
    };
    let result = ToolFunction::execute(&tool, params).await.unwrap();
    assert_eq!(result.result, "Required: test, Optional: None");
}

#[test]
fn test_optional_schema() {
    let schema = <HandleOptional as ToolFunction>::get_schema();
    let params = schema.function.parameters.unwrap();

    // Check that optional parameter is marked as not required
    let properties = params.get("properties").unwrap();
    let optional = properties.get("optional").unwrap();
    assert_eq!(optional.get("required"), Some(&serde_json::json!(false)));
}
