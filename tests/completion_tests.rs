// tests/chat_client.rs
use std::sync::Arc;
use std::time::SystemTime;
use async_openai::types::CreateChatCompletionResponse;
use futures::StreamExt;
use mockall::mock;
use mockall::predicate::*;
use uuid::Uuid;
use std::io::{stdout, Write};

use agentgraph::completion::*;

const TEST_MODEL: &str = "gpt-4-turbo-preview";

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
            input: String,
            output: CreateChatCompletionResponse,
        );
        
        async fn record_stream_span(
            &self,
            trace_id: Uuid,
            name: String,
            start_time: SystemTime,
            end_time: SystemTime,
            input: String,
            output: String,
        );
    }
}

#[tokio::test]
async fn test_chat_completion() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    
    // Create client without tracing
    let client = ChatClientImpl::new(api_key.clone());
    
    // Test simple completion
    let content = "Say 'test response' exactly";
    
    let response = client
        .chat_completion(TEST_MODEL, content)
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
    
    let content = "Say 'traced test' exactly";
    
    let response = client_with_tracing
        .chat_completion(TEST_MODEL, content)
        .await
        .expect("Chat completion failed");
    
    assert!(response.choices[0].message.content
        .as_ref()
        .unwrap()
        .contains("traced test"));
}

#[tokio::test]
async fn test_chat_completion_stream() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    
    // Create client without tracing
    let client = ChatClientImpl::new(api_key.clone());
    
    // Test streaming
    let content = "Count from 1 to 3";
    
    let mut stream = client
        .chat_completion_stream(TEST_MODEL, content)
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
    
    let content = "Count from 4 to 6";
    
    let mut stream = client_with_tracing
        .chat_completion_stream(TEST_MODEL, content)
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