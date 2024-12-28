use agentgraph_core::prelude::*;
use agentgraph_macros::tools;
use serde::{Deserialize, Serialize};

/// Simple data for add-params
 pub struct AddParams {
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

/// Simple data for add-response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddResponse {
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

/// Another set of parameters for subtract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubParams {
    a: i32,
    b: i32,
}

impl JsonSchema for SubParams {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer"},
                "b": {"type": "integer"}
            },
            "required": ["a", "b"]
        })
    }
}

/// Another response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubResponse {
    diff: i32,
}

impl JsonSchema for SubResponse {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "diff": {"type": "integer"}
            },
            "required": ["diff"]
        })
    }
}

/// This is our "tool" struct
#[derive(Clone)]
struct MathTool;

/// We apply our #[tools(...)] macro here, listing each method name = "description"
#[tools(add = "Adds two numbers", subtract = "Subtracts second from first")]
impl MathTool {
    async fn add(&self, params: AddParams) -> std::result::Result<AddResponse, ToolError> {
        Ok(AddResponse {
            sum: params.x + params.y,
        })
    }

    async fn subtract(&self, params: SubParams) -> std::result::Result<SubResponse, ToolError> {
        Ok(SubResponse {
            diff: params.a - params.b,
        })
    }

    // Some other methods might be ignored, as they're not in the attribute list
    async fn helper(&self, x: i32) -> i32 {
        x * x
    }
}

//
// Because the macro expands into two new tool types (e.g. MathToolAdd, MathToolSubtract)
// implementing ToolFunction, we can test them individually.
//

#[tokio::test]
async fn test_add_tool_execution() {
    // The macro expansion created a struct named "MathToolAdd"
    // implementing ToolFunction<Params = AddParams, Response = AddResponse>.
    let tool = MathToolAdd(MathTool);

    // Confirm that name/description are as we specified
    assert_eq!(<MathToolAdd as ToolFunction>::name(), "add");
    assert_eq!(
        <MathToolAdd as ToolFunction>::description(),
        "Adds two numbers"
    );

    let result = ToolFunction::execute(&tool, AddParams { x: 7, y: 3 })
        .await
        .unwrap();
    assert_eq!(result.sum, 10);
}

#[test]
fn test_add_tool_schema() {
    // If you have get_schema() or similar
    let schema = <MathToolAdd as ToolFunction>::get_schema();

    // Check top-level metadata
    assert_eq!(schema.function.name, "add");
    assert_eq!(
        schema.function.description.as_ref().unwrap(),
        "Adds two numbers"
    );

    // Check parameter schema
    let params = schema.function.parameters.as_ref().unwrap();
    assert_eq!(
        params,
        &serde_json::json!({
            "type": "object",
            "properties": {
                "x": {"type": "integer"},
                "y": {"type": "integer"}
            },
            "required": ["x", "y"]
        })
    );
}

#[tokio::test]
async fn test_subtract_tool_execution() {
    // The macro also created "MathToolSubtract"
    let tool = MathToolSubtract(MathTool);

    assert_eq!(<MathToolSubtract as ToolFunction>::name(), "subtract");
    assert_eq!(
        <MathToolSubtract as ToolFunction>::description(),
        "Subtracts second from first"
    );

    let result = ToolFunction::execute(&tool, SubParams { a: 10, b: 3 })
        .await
        .unwrap();
    assert_eq!(result.diff, 7);
}

#[test]
fn test_subtract_tool_schema() {
    let schema = <MathToolSubtract as ToolFunction>::get_schema();
    assert_eq!(schema.function.name, "subtract");
    assert_eq!(
        schema.function.description.as_ref().unwrap(),
        "Subtracts second from first"
    );

    let params = schema.function.parameters.as_ref().unwrap();
    assert_eq!(
        params,
        &serde_json::json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer"},
                "b": {"type": "integer"}
            },
            "required": ["a", "b"]
        })
    );
}
