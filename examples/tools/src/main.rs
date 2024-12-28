use agentgraph_core::{ToolError, ToolFunction};
use agentgraph_macros::tools;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::to_string;

// Our data types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddParams {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddResponse {
    sum: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubParams {
    a: i32,
    b: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubResponse {
    diff: i32,
}

// The "tool" struct
#[derive(Clone)]
struct MathTool;

// Apply the macro, listing methods = descriptions
#[tools(add = "Adds two numbers", subtract = "Subtracts second from first")]
impl MathTool {
    // If we return `Result<AddResponse, ToolError>`, the macro
    // uses `.await?`.
    // If we return a plain type, it uses `.await`.
    async fn add(&self, params: AddParams) -> Result<AddResponse, ToolError> {
        Ok(AddResponse {
            sum: params.x + params.y,
        })
    }

    async fn subtract(&self, params: SubParams) -> Result<SubResponse, ToolError> {
        // Return a plain type, so the macro uses `.await` (no `?`)
        Ok(SubResponse {
            diff: params.a - params.b,
        })
    }

    async fn _helper(&self, val: i32) -> i32 {
        val * 10
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Because the macro expansions create `MathToolAdd` and `MathToolSubtract`,
    // we can do:
    let add_tool = MathToolAdd(MathTool);
    let add_tool_schema = <MathToolAdd as ToolFunction>::get_schema();
    let sub_tool = MathToolSubtract(MathTool);
    let sub_tool_schema = <MathToolSubtract as ToolFunction>::get_schema();

    // Now we can call the macro-generated trait:
    let add_res = ToolFunction::execute(&add_tool, AddParams { x: 3, y: 4 }).await?;
    println!("Add name: {}", <MathToolAdd as ToolFunction>::name());
    println!(
        "Add description: {}",
        <MathToolAdd as ToolFunction>::description()
    );
    println!("Add result: {}", add_res.sum);
    println!("Add schema: {}", to_string(&add_tool_schema)?);

    let sub_res = ToolFunction::execute(&sub_tool, SubParams { a: 7, b: 2 }).await?;
    println!(
        "Subtract name: {}",
        <MathToolSubtract as ToolFunction>::name()
    );
    println!(
        "Subtract description: {}",
        <MathToolSubtract as ToolFunction>::description()
    );
    println!("Subtract result: {}", sub_res.diff);
    println!("Subtract schema: {}", to_string(&sub_tool_schema)?);

    Ok(())
}
