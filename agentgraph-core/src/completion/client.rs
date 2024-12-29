use crate::{TracingError, TracingProvider};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionTool, ChatCompletionToolChoiceOption,
        CreateChatCompletionRequest, CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
        CreateChatCompletionStreamResponse,
    },
    Client as OpenAIClient,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ChatCompletionRequestOptions {
    pub model: String,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<ChatCompletionTool>>,
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,
}

const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_TEMPERATURE: f32 = 0.0;

impl Default for ChatCompletionRequestOptions {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            temperature: DEFAULT_TEMPERATURE.into(),
            tools: None,
            tool_choice: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatCompletionCallOptions {
    pub trace_id: Option<String>,
    pub parent_trace_id: Option<String>,
}

impl ChatCompletionCallOptions {
    pub fn new(trace_id: Option<String>, parent_trace_id: Option<String>) -> Self {
        Self {
            trace_id,
            parent_trace_id,
        }
    }
}

#[async_trait]
pub trait ChatClient: Send + Sync {
    // Request creation methods
    fn create_chat_completion_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        options: ChatCompletionRequestOptions,
    ) -> Result<CreateChatCompletionRequest, Box<dyn std::error::Error + Send + Sync>>;

    fn create_chat_completion_stream_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        options: ChatCompletionRequestOptions,
    ) -> Result<CreateChatCompletionRequest, Box<dyn std::error::Error + Send + Sync>>;

    // Completion methods
    async fn complete(
        &self,
        request: CreateChatCompletionRequest,
        options: Option<ChatCompletionCallOptions>,
    ) -> Result<CreateChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>>;

    async fn complete_stream(
        &self,
        request: CreateChatCompletionRequest,
        options: Option<ChatCompletionCallOptions>,
    ) -> Result<
        Pin<
            Box<
                dyn Stream<
                        Item = Result<
                            CreateChatCompletionStreamResponse,
                            Box<dyn std::error::Error + Send + Sync>,
                        >,
                    > + Send,
            >,
        >,
        Box<dyn std::error::Error + Send + Sync>,
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

    // Base function to create request builder with common options
    fn create_base_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        options: ChatCompletionRequestOptions,
    ) -> CreateChatCompletionRequestArgs {
        let mut builder = CreateChatCompletionRequestArgs::default();
        let mut builder = builder.model(options.model);
        let mut builder = builder.messages(messages);
        let mut builder = if let Some(temp) = options.temperature {
            builder.temperature(temp)
        } else {
            builder
        };
        let mut builder = if let Some(tools) = options.tools {
            builder.tools(tools)
        } else {
            builder
        };
        let builder = if let Some(tool_choice) = options.tool_choice {
            builder.tool_choice(tool_choice)
        } else {
            builder
        };

        builder.to_owned()
    }
}

#[async_trait]
impl ChatClient for ChatClientImpl {
    fn create_chat_completion_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        options: ChatCompletionRequestOptions,
    ) -> Result<CreateChatCompletionRequest, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .create_base_request(messages, options)
            .stream(false)
            .build()?
            .into())
    }

    fn create_chat_completion_stream_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        options: ChatCompletionRequestOptions,
    ) -> Result<CreateChatCompletionRequest, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .create_base_request(messages, options)
            .stream(true)
            .build()?
            .into())
    }

    async fn complete(
        &self,
        request: CreateChatCompletionRequest,
        options: Option<ChatCompletionCallOptions>,
    ) -> Result<CreateChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let trace_id = options
            .as_ref()
            .and_then(|o| o.trace_id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let parent_trace_id = options.as_ref().and_then(|o| o.parent_trace_id.clone());

        // If we want to capture the entire request:
        let inputs = serde_json::to_value(&request)
            .unwrap_or_else(|_| json!({ "error": "Failed to serialize request" }));

        // Start trace
        if let Some(tracer) = &self.tracer {
            tracer
                .start_trace(
                    &trace_id,
                    "chat_completion",
                    "llm",
                    &inputs,
                    parent_trace_id,
                    Some(SystemTime::now()),
                )
                .await?;
        }

        // Call the OpenAI endpoint
        let response = self.client.chat().create(request.clone()).await?;

        // End trace
        if let Some(tracer) = &self.tracer {
            let outputs = serde_json::to_value(&response)
                .unwrap_or_else(|_| json!({ "error": "Failed to serialize response" }));

            tracer
                .end_trace(&trace_id, &outputs, Some(SystemTime::now()))
                .await?;
        }

        Ok(response)
    }

    async fn complete_stream(
        &self,
        request: CreateChatCompletionRequest,
        options: Option<ChatCompletionCallOptions>,
    ) -> Result<
        Pin<
            Box<
                dyn futures::Stream<
                        Item = Result<
                            CreateChatCompletionStreamResponse,
                            Box<dyn std::error::Error + Send + Sync>,
                        >,
                    > + Send,
            >,
        >,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let trace_id = options
            .as_ref()
            .and_then(|o| o.trace_id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let parent_trace_id = options.as_ref().and_then(|o| o.parent_trace_id.clone());

        // Serialize the entire request for the trace
        let inputs = serde_json::to_value(&request)
            .unwrap_or_else(|_| json!({ "error": "Failed to serialize request" }));

        // Start trace
        if let Some(tracer) = &self.tracer {
            tracer
                .start_trace(
                    &trace_id,
                    "chat_completion_stream",
                    "chain",
                    &inputs,
                    parent_trace_id,
                    Some(SystemTime::now()),
                )
                .await?;
        }

        let mut stream = self.client.chat().create_stream(request).await?;
        let tracer = self.tracer.clone();

        let stream = async_stream::stream! {
            let mut full_response = String::new();
            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        // Collect streamed content
                        if let Some(choice) = response.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                full_response.push_str(content);
                            }
                        }
                        yield Ok(response);
                    }
                    Err(e) => {
                        yield Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
                    }
                }
            }

            // End trace after we finish streaming
            if let Some(tracer) = tracer {
                let outputs = json!({ "streamed_content": full_response });
                if let Err(e) = tracer
                    .end_trace(
                        &trace_id,
                        &outputs,
                        Some(SystemTime::now()),
                    )
                    .await
                {
                    eprintln!("Error ending stream trace: {:?}", e);
                }
            }
        };

        Ok(Box::pin(stream))
    }
}
