use async_openai::{
    types::{
        CreateChatCompletionRequestArgs,
        CreateChatCompletionRequest,
        CreateChatCompletionResponse,
        CreateChatCompletionStreamResponse,
        ChatCompletionRequestMessage,
    },
    Client as OpenAIClient,
    config::OpenAIConfig,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::sync::Arc;
use std::pin::Pin;
use std::time::SystemTime;
use uuid::Uuid;

#[async_trait]
pub trait TracingProvider: Send + Sync {
    async fn record_span(
        &self,
        trace_id: Uuid,
        name: String,
        start_time: SystemTime,
        end_time: SystemTime,
        messages: Vec<ChatCompletionRequestMessage>,
        output: CreateChatCompletionResponse,
    );
    
    async fn record_stream_span(
        &self,
        trace_id: Uuid,
        name: String,
        start_time: SystemTime,
        end_time: SystemTime,
        messages: Vec<ChatCompletionRequestMessage>,
        output: String,
    );
}

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<CreateChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>>;

    async fn chat_completion_stream(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>
    >;
}

pub struct ChatClientImpl {
    client: OpenAIClient<OpenAIConfig>,
    tracer: Option<Arc<dyn TracingProvider>>,
}

impl ChatClientImpl {
    pub fn new(api_key: String) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = OpenAIClient::with_config(config);
        Self { 
            client,
            tracer: None,
        }
    }

    pub fn with_tracer(self, tracer: Arc<dyn TracingProvider>) -> Self {
        Self {
            client: self.client,
            tracer: Some(tracer),
        }
    }

    // Helper function to create a completion request
    fn create_completion_request(
        &self,
        model: &str, 
        messages: Vec<ChatCompletionRequestMessage>,
        stream: bool,
    ) -> Result<CreateChatCompletionRequest, Box<dyn std::error::Error + Send + Sync>> {
        Ok(CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages)
            .stream(stream)
            .build()?
            .into())
    }
}

#[async_trait]
impl ChatClient for ChatClientImpl {
    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<CreateChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = SystemTime::now();
        let trace_id = Uuid::new_v4();

        let request = self.create_completion_request(model, messages.clone(), false)?;
        let response = self.client.chat().create(request).await?;
        let end_time = SystemTime::now();

        if let Some(tracer) = &self.tracer {
            tracer
                .record_span(
                    trace_id,
                    "chat_completion".into(),
                    start_time,
                    end_time,
                    messages,
                    response.clone(),
                )
                .await;
        }

        Ok(response)
    }

    async fn chat_completion_stream(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>
    > {
        let start_time = SystemTime::now();
        let trace_id = Uuid::new_v4();
        let tracer = self.tracer.clone();
        let messages_clone = messages.clone();

        let request = self.create_completion_request(model, messages, true)?;
        let mut stream = self.client.chat().create_stream(request).await?;

        let stream = async_stream::stream! {
            let mut full_response = String::new();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        if let Some(delta) = response.choices.first() {
                            if let Some(content) = &delta.delta.content {
                                full_response.push_str(content);
                            }
                        }
                        yield Ok(response);
                    }
                    Err(e) => yield Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
                }
            }

            if let Some(tracer) = tracer {
                let end_time = SystemTime::now();
                tracer
                    .record_stream_span(
                        trace_id,
                        "chat_completion_stream".into(),
                        start_time,
                        end_time,
                        messages_clone,
                        full_response,
                    )
                    .await;
            }
        };

        Ok(Box::pin(stream))
    }
}