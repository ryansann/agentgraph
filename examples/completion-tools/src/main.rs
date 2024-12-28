use agentgraph_core::prelude::*;
use agentgraph_macros::tools;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    ChatCompletionToolChoiceOption,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;

// Weather tool types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WeatherParams {
    #[schemars(description = "Location to check the weather")]
    location: String,
    #[schemars(description = "Temperature unit (F or C)")]
    unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherResponse {
    temperature: f32,
    conditions: String,
}

// Time conversion tool types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimeConversionParams {
    #[schemars(description = "Time to convert")]
    time: String,
    #[schemars(description = "Time zone to convert from")]
    from_zone: String,
    #[schemars(description = "Time zone to convert to")]
    to_zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConversionResponse {
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
    async fn get_weather(
        &self,
        params: WeatherParams,
    ) -> std::result::Result<WeatherResponse, ToolError> {
        // Mock weather implementation
        Ok(WeatherResponse {
            temperature: 72.0,
            conditions: format!("Sunny in {}", params.location),
        })
    }

    async fn convert_time(
        &self,
        params: TimeConversionParams,
    ) -> Result<TimeConversionResponse, ToolError> {
        // Mock time conversion implementation
        Ok(TimeConversionResponse {
            converted_time: format!(
                "Converted {} from {} to {}",
                params.time, params.from_zone, params.to_zone
            ),
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
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

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

    let weather_schema = <AssistantToolsGetWeather as ToolFunction>::get_schema();
    println!(
        "Get Weather Schema: {}",
        serde_json::to_string_pretty(&weather_schema)?
    );

    let time_schema = <AssistantToolsConvertTime as ToolFunction>::get_schema();
    println!(
        "Convert Time Schema: {}",
        serde_json::to_string_pretty(&time_schema)?
    );

    // Create tools vector for the API request
    let tools = vec![weather_schema, time_schema];

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
        println!(
            "\nAssistant: {}",
            choice.message.content.unwrap_or_default()
        );

        // Handle any tool calls
        if let Some(tool_calls) = choice.message.tool_calls {
            let get_weather_name = <AssistantToolsGetWeather as ToolFunction>::name();
            let convert_time_name = <AssistantToolsConvertTime as ToolFunction>::name();
            for tool_call in tool_calls {
                match tool_call.function.name.as_str() {
                    name if name == get_weather_name => {
                        let params: WeatherParams =
                            serde_json::from_str(&tool_call.function.arguments)?;
                        let weather = ToolFunction::execute(&weather_tool, params).await?;
                        println!("\nWeather Tool Response:");
                        println!("Temperature: {}°F", weather.temperature);
                        println!("Conditions: {}", weather.conditions);
                    }
                    name if name == convert_time_name => {
                        let params: TimeConversionParams =
                            serde_json::from_str(&tool_call.function.arguments)?;
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
