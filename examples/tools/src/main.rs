use agentgraph_macros::tools; // <-- the macro
use async_trait::async_trait;
use agentgraph::{
    ToolError,
    ToolFunction,
};
use serde::{Serialize, Deserialize};
use serde_json::to_string;

// Our data types
#[derive(Clone, Serialize, Deserialize)]
struct AddParams {
    x: i32,
    y: i32,
}

#[derive(Clone, Serialize, Deserialize)]
struct AddResponse {
    sum: i32,
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
        Ok(AddResponse { sum: params.x + params.y })
    }

    async fn subtract(&self, params: (i32, i32)) -> (i32) {
        // Return a plain type, so the macro uses `.await` (no `?`)
        (params.0 - params.1)
    }

    async fn helper(&self, val: i32) -> i32 {
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
    println!("Add result: {}", add_res.sum);
    println!("Add schema: {}", to_string(&add_tool_schema)?);

    let sub_res = ToolFunction::execute(&sub_tool, (7, 2)).await?;
    println!("Subtract result: {}", sub_res);
    println!("Subtract schema: {}", to_string(&sub_tool_schema)?);

    Ok(())
}
