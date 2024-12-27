use std::sync::Arc;
use std::env;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};

use agentgraph_core::{
    ChatClient,
    ChatClientImpl,
    ChatCompletionRequestOptions,
    LangSmithTracer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {  // Changed error type here
    // Get API keys from environment
    let openai_api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");
    let langsmith_api_key = env::var("LANGSMITH_API_KEY")
        .expect("LANGSMITH_API_KEY must be set");

    // Initialize client with LangSmith tracing
    let tracer = Arc::new(LangSmithTracer::new(langsmith_api_key));
    let client = ChatClientImpl::new(openai_api_key).with_tracer(tracer);

    // Create messages using the builder pattern
    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content("You are a helpful assistant.")
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content("What is the capital of France?")
            .build()?
            .into(),
    ];

    let options = ChatCompletionRequestOptions {
        model: "gpt-4o-mini".to_string(),
        temperature: Some(0.0),
        ..Default::default()
    };

    println!("Creating chat completion request...");
    let request = client.create_chat_completion_request(messages, options)?;
    
    // Print the request for inspection
    println!("\nRequest:\n{}", serde_json::to_string_pretty(&request)?);

    println!("\nSending request to OpenAI...");
    let response = client.complete(request, None).await?;

    println!("\nResponse:");
    for choice in response.choices {
        println!(
            "{}: Role: {}  Content: {:?}",
            choice.index, choice.message.role, choice.message.content
        );
    }

    // Print usage information
    if let Some(usage) = response.usage {
        println!("\nToken usage:");
        println!("  Prompt tokens: {}", usage.prompt_tokens);
        println!("  Completion tokens: {}", usage.completion_tokens);
        println!("  Total tokens: {}", usage.total_tokens);
    }

    Ok(())
}