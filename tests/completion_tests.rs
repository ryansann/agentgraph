// tests/chat_client.rs
use std::sync::Arc;
use std::time::SystemTime;
use async_openai::types::{
    CreateChatCompletionResponse,
    ChatCompletionRequestMessage,
    ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequest,
};
use futures::StreamExt;
use mockall::mock;
use mockall::predicate::*;
use uuid::Uuid;
use std::io::{stdout, Write};

use agentgraph::completion::*;

const TEST_MODEL: &str = "gpt-4o-mini";

// Mock tracer for testing
mock! {
    pub TracerTest {}
    #[async_trait::async_trait]
    impl TracingProvider for TracerTest {
        async fn record_span(
            &self,
            trace_id: Uuid,
            name: String,
            start_time: SystemTime,
            end_time: SystemTime,
            request: &CreateChatCompletionRequest,
            output: CreateChatCompletionResponse,
        );
        
        async fn record_stream_span(
            &self,
            trace_id: Uuid,
            name: String,
            start_time: SystemTime,
            end_time: SystemTime,
            request: &CreateChatCompletionRequest,
            output: String,
        );
    }
}

fn create_test_message(content: &str) -> Vec<ChatCompletionRequestMessage> {
    let message = ChatCompletionRequestUserMessageArgs::default()
        .content(content)
        .build()
        .expect("Failed to build message");
    vec![ChatCompletionRequestMessage::User(message)]
}

fn create_test_options(model: Option<String>) -> ChatCompletionRequestOptions {
    ChatCompletionRequestOptions {
        model: model.unwrap_or_else(|| TEST_MODEL.to_string()),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_chat_completion() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    
    // Create client without tracing
    let client = ChatClientImpl::new(api_key.clone());
    
    // Test simple completion
    let messages = create_test_message("Say 'test response' exactly");
    let request = client
        .create_chat_completion_request(messages, create_test_options(None))
        .expect("Failed to create request");
    
    let response = client
        .complete(request, None)
        .await
        .expect("Chat completion failed");
    
    assert!(response.choices[0].message.content
        .as_ref()
        .unwrap()
        .contains("test response"));

    // Test with tracing
    let mut mock_tracer = MockTracerTest::new();
    mock_tracer
        .expect_record_span()
        .times(1)
        .returning(|_, _, _, _, _, _| ());
    
        let client_with_tracing = client.with_tracer(Arc::new(mock_tracer));
    
        let messages = create_test_message("Say 'hello' exactly");
        let request = client_with_tracing
            .create_chat_completion_request(messages, create_test_options(None))
            .expect("Failed to create request");
    
        let trace_id = Uuid::new_v4();
        let options = ChatCompletionCallOptions {
            trace_id: Some(trace_id),
        };
        
        let response = client_with_tracing
            .complete(request, Some(options))
            .await
            .expect("Chat completion failed");
        
        assert!(response.choices[0].message.content.is_some(), "Response content should not be None");
        // The model should generate some response, but we won't check for exact text
        assert!(!response.choices[0].message.content.as_ref().unwrap().is_empty(), "Response should not be empty");
}

#[tokio::test]
async fn test_chat_completion_stream() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    
    // Create client without tracing
    let client = ChatClientImpl::new(api_key.clone());
    
    // Test streaming
    let messages = create_test_message("Count from 1 to 3");
    let request = client
        .create_chat_completion_stream_request(messages, create_test_options(None))
        .expect("Failed to create request");
    
    let mut stream = client
        .complete_stream(request, None)
        .await
        .expect("Failed to create stream");

    let mut lock = stdout().lock();
    let mut full_response = String::new();

    while let Some(response) = stream.next().await {
        match response {
            Ok(response) => {
                response.choices.iter().for_each(|chat_choice| {
                    if let Some(ref content) = chat_choice.delta.content {
                        write!(lock, "{}", content).unwrap();
                        full_response.push_str(content);
                    }
                });
            }
            Err(e) => panic!("Stream error: {}", e),
        }
        stdout().flush().unwrap();
    }
    
    assert!(full_response.contains("1"));
    assert!(full_response.contains("2"));
    assert!(full_response.contains("3"));

    // Test streaming with tracing
    let mut mock_tracer = MockTracerTest::new();
    mock_tracer
        .expect_record_stream_span()
        .times(1)
        .returning(|_, _, _, _, _, _| ());
    
    let client_with_tracing = client.with_tracer(Arc::new(mock_tracer));
    
    let messages = create_test_message("Count from 4 to 6");
    let request = client_with_tracing
        .create_chat_completion_stream_request(messages, create_test_options(None))
        .expect("Failed to create request");

    let trace_id = Uuid::new_v4();
    let options = ChatCompletionCallOptions {
        trace_id: Some(trace_id),
    };
    
    let mut stream = client_with_tracing
        .complete_stream(request, Some(options))
        .await
        .expect("Failed to create stream");

    let mut lock = stdout().lock();
    let mut full_response = String::new();

    while let Some(response) = stream.next().await {
        match response {
            Ok(response) => {
                response.choices.iter().for_each(|chat_choice| {
                    if let Some(ref content) = chat_choice.delta.content {
                        write!(lock, "{}", content).unwrap();
                        full_response.push_str(content);
                    }
                });
            }
            Err(e) => panic!("Stream error: {}", e),
        }
        stdout().flush().unwrap();
    }
    
    assert!(full_response.contains("4"));
    assert!(full_response.contains("5"));
    assert!(full_response.contains("6"));
}

#[tokio::test]
async fn test_request_creation() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let client = ChatClientImpl::new(api_key);
    
    let messages = create_test_message("test");
    let options = ChatCompletionRequestOptions {
        model: TEST_MODEL.to_string(),
        temperature: Some(0.7),
        tools: None,
        tool_choice: None,
    };

    // Test normal request creation
    let request = client.create_chat_completion_request(messages.clone(), options.clone())
        .expect("Failed to create request");
    assert!(!request.stream.unwrap_or(true));
    assert_eq!(request.model, TEST_MODEL);
    
    // Test stream request creation
    let request = client.create_chat_completion_stream_request(messages, options)
        .expect("Failed to create request");
    assert!(request.stream.unwrap_or(false));
    assert_eq!(request.model, TEST_MODEL);
}
