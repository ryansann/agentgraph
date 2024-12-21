use async_openai::{
    types::{
        CreateChatCompletionRequestArgs,
        CreateChatCompletionRequest,
        CreateChatCompletionResponse,
        CreateChatCompletionStreamResponse,
        ChatCompletionRequestMessage,
        ChatCompletionTool,
        ChatCompletionToolChoiceOption,
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

#[derive(Debug, Clone)]
pub struct ChatCompletionRequestOptions {
    pub model: String,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<ChatCompletionTool>>,
    pub tool_choice: Option<ChatCompletionToolChoiceOption>,
}

impl Default for ChatCompletionRequestOptions {
    fn default() -> Self {
        Self {
            model: "gpt-4-turbo-preview".to_string(),
            temperature: None,
            tools: None,
            tool_choice: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatCompletionCallOptions {
    pub trace_id: Option<Uuid>,
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
        Ok(self.create_base_request(messages, options)
            .stream(false)
            .build()?
            .into())
    }

    fn create_chat_completion_stream_request(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        options: ChatCompletionRequestOptions,
    ) -> Result<CreateChatCompletionRequest, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.create_base_request(messages, options)
            .stream(true)
            .build()?
            .into())
    }

    async fn complete(
        &self,
        request: CreateChatCompletionRequest,
        options: Option<ChatCompletionCallOptions>,
    ) -> Result<CreateChatCompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = SystemTime::now();
        let trace_id = options
            .and_then(|o| o.trace_id)
            .unwrap_or_else(Uuid::new_v4);

        let response = self.client.chat().create(request.clone()).await?;
        let end_time = SystemTime::now();

        if let Some(tracer) = &self.tracer {
            tracer
                .record_span(
                    trace_id,
                    "chat_completion".into(),
                    start_time,
                    end_time,
                    &request,
                    response.clone(),
                )
                .await;
        }

        Ok(response)
    }

    async fn complete_stream(
        &self,
        request: CreateChatCompletionRequest,
        options: Option<ChatCompletionCallOptions>,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CreateChatCompletionStreamResponse, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>
    > {
        let start_time = SystemTime::now();
        let trace_id = options
            .and_then(|o| o.trace_id)
            .unwrap_or_else(Uuid::new_v4);
        let tracer = self.tracer.clone();
        let request_clone = request.clone();

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
                        &request_clone,
                        full_response,
                    )
                    .await;
            }
        };

        Ok(Box::pin(stream))
    }
}