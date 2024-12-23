use agentgraph::prelude::*;
use agentgraph_macros::tools;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    ChatCompletionToolChoiceOption,
};
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;
use std::env;

// Weather tool types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    location: String,
    unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WeatherResponse {
    temperature: f32,
    conditions: String,
}

// Time conversion tool types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct TimeConversionParams {
    time: String,
    from_zone: String,
    to_zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimeConversionResponse {
    converted_time: String,
}

// Our combined tools struct
#[derive(Clone)]
struct AssistantTools;

// Apply the tools macro with multiple function descriptions
#[tools(
    get_weather = "Get the current weather for a location",
    convert_time = "Convert time between different time zones"
)]
impl AssistantTools {
    async fn get_weather(&self, params: WeatherParams) -> Result<WeatherResponse, ToolError> {
        // Mock weather implementation
        Ok(WeatherResponse {
            temperature: 72.0,
            conditions: format!("Sunny in {}", params.location),
        })
    }

    async fn convert_time(&self, params: TimeConversionParams) -> Result<TimeConversionResponse, ToolError> {
        // Mock time conversion implementation
        Ok(TimeConversionResponse {
            converted_time: format!("Converted {} from {} to {}", 
                params.time, params.from_zone, params.to_zone),
        })
    }

    // Helper method - not exposed as a tool since it's not in the macro
    async fn internal_helper(&self, value: i32) -> i32 {
        value * 2
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get API key from environment
    let openai_api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");

    // Create the chat client
    let client = ChatClientImpl::new(openai_api_key);

    // Create our tool instances
    // The macro creates AssistantToolsGetWeather and AssistantToolsConvertTime
    let weather_tool = AssistantToolsGetWeather(AssistantTools);
    let time_tool = AssistantToolsConvertTime(AssistantTools);

    // Create the messages
    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content("You are a helpful assistant that can check weather and convert time zones.")
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content("What's the weather like in San Francisco, and what time is it there if it's 2PM in New York?")
            .build()?
            .into(),
    ];

    // Get schemas for both tools
    let tools = vec![
        AssistantToolsGetWeather::get_schema(),
        AssistantToolsConvertTime::get_schema(),
    ];

    // Create request options with tools
    let options = ChatCompletionRequestOptions {
        model: "gpt-4".to_string(),
        temperature: Some(0.0),
        tools: Some(tools),
        tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
    };

    // Create and send the request
    println!("Creating chat completion request...");
    let request = client.create_chat_completion_request(messages, options)?;
    
    println!("\nSending request to OpenAI...");
    let response = client.complete(request, None).await?;

    // Process the response and tool calls
    for choice in response.choices {
        println!("\nAssistant: {}", choice.message.content.unwrap_or_default());
        
        // Handle any tool calls
        if let Some(tool_calls) = choice.message.tool_calls {
            for tool_call in tool_calls {
                match tool_call.function.name.as_str() {
                    "get_weather" => {
                        let params: WeatherParams = serde_json::from_str(&tool_call.function.arguments)?;
                        let weather = ToolFunction::execute(&weather_tool, params).await?;
                        println!("\nWeather Tool Response:");
                        println!("Temperature: {}°F", weather.temperature);
                        println!("Conditions: {}", weather.conditions);
                    }
                    "convert_time" => {
                        let params: TimeConversionParams = serde_json::from_str(&tool_call.function.arguments)?;
                        let time = ToolFunction::execute(&time_tool, params).await?;
                        println!("\nTime Conversion Tool Response:");
                        println!("Converted Time: {}", time.converted_time);
                    }
                    _ => println!("Unknown tool called: {}", tool_call.function.name),
                }
            }
        }
    }

    Ok(())
}