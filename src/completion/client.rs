use async_openai::{
    types::{
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
        CreateChatCompletionResponse, 
        CreateChatCompletionStreamResponse,
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

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn chat_completion(
        &self,
        model: &str,
        content: &str,
    ) -> Result<CreateChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>>;

    async fn chat_completion_stream(
        &self,
        model: &str,
        content: &str,
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
}

#[async_trait]
impl ChatClient for ChatClientImpl {
    async fn chat_completion(
        &self,
        model: &str,
        content: &str,
    ) -> Result<CreateChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = SystemTime::now();
        let trace_id = Uuid::new_v4();

        let message = ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages([message.into()])
            .build()?;

        let response = self.client.chat().create(request).await?;
        let end_time = SystemTime::now();

        if let Some(tracer) = &self.tracer {
            tracer
                .record_span(
                    trace_id,
                    "chat_completion".into(),
                    start_time,
                    end_time,
                    content.to_string(),
                    response.clone(),
                )
                .await;
        }

        Ok(response)
    }

    async fn chat_completion_stream(
        &self,
        model: &str,
        content: &str,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>
    > {
        let start_time = SystemTime::now();
        let trace_id = Uuid::new_v4();
        let tracer = self.tracer.clone();
        let content = content.to_string();

        let message = ChatCompletionRequestUserMessageArgs::default()
            .content(content.clone())
            .build()?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages([message.into()])
            .stream(true)
            .build()?;

        let mut stream = self.client.chat().create_stream(request).await?;

        let stream = async_stream::stream! {
            let mut full_response = String::new();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        if let Some(content) = &response.choices[0].delta.content {
                            full_response.push_str(content);
                        }
                        yield Ok(response);
                    }
                    Err(e) => yield Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
                }
            }

            // Record the complete span after stream ends
            if let Some(tracer) = tracer {
                let end_time = SystemTime::now();
                tracer
                    .record_stream_span(
                        trace_id,
                        "chat_completion_stream".into(),
                        start_time,
                        end_time,
                        content,
                        full_response,
                    )
                    .await;
            }
        };

        Ok(Box::pin(stream))
    }
}