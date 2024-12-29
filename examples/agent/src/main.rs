use agentgraph_core::prelude::*;
use agentgraph_macros::{tools, State};
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestToolMessageArgs, ChatCompletionToolChoiceOption,
};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

// Search tool types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchParams {
    #[schemars(description = "The search query to execute")]
    query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    results: Vec<SearchResult>,
}

// Our search agent's tools
#[derive(Clone)]
struct SearchTools;

// Apply the tools macro
#[tools(web_search = "Search the web and return a list of relevant results")]
impl SearchTools {
    async fn web_search(&self, params: SearchParams) -> Result<SearchResponse, ToolError> {
        // Mock search implementation
        let result = SearchResult {
            title: "Example Search Result".to_string(),
            url: "https://example.com".to_string(),
            snippet: format!("Relevant information about: {}", params.query),
        };

        let results = vec![result];

        Ok(SearchResponse { results })
    }
}

#[derive(State, Debug, Clone, Serialize, Deserialize)]
pub struct SearchAgentState {
    #[update(append)]
    messages: Vec<ChatCompletionRequestMessage>,

    #[update(append)]
    errors: Vec<String>,
}

impl SearchAgentState {
    pub fn new(system: Option<String>, user: Option<String>) -> Self {
        let mut messages = vec![];
        if let Some(msg) = system {
            messages.push(ChatCompletionRequestMessage::System(msg.into()));
        }
        if let Some(msg) = user {
            messages.push(ChatCompletionRequestMessage::User(msg.into()));
        }
        Self {
            messages: messages,
            errors: Vec::new(),
        }
    }

    pub fn latest_message_has_tool_calls(&self) -> bool {
        self.messages.last().map_or(false, |msg| {
            match msg {
                ChatCompletionRequestMessage::Assistant(asst_msg) => {
                    // Check if the message has any tool calls
                    asst_msg
                        .tool_calls
                        .as_ref()
                        .map_or(false, |calls| !calls.is_empty())
                }
                // Other message types (System, User) can't have tool calls
                _ => false,
            }
        })
    }

    pub fn get_latest_messages(&self, count: usize) -> Vec<ChatCompletionRequestMessage> {
        let mut result = Vec::new();

        // Add up to `count` most recent non-system messages
        result.extend(
            self.messages
                .iter()
                .rev() // Reverse to get most recent first
                .filter(|msg| !matches!(msg, ChatCompletionRequestMessage::System(_)))
                .take(count)
                .collect::<Vec<_>>() // collect so we can reverse again
                .into_iter()
                .rev() // restore chronological order
                .cloned(),
        );

        result
    }
}

impl Default for SearchAgentState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            errors: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct AgentNode {
    agent: Arc<SearchAgent>,
    name: String,
}

#[async_trait]
impl Node<SearchAgentState> for AgentNode {
    async fn process(
        &self,
        ctx: &Context,
        state: SearchAgentState,
    ) -> NodeResult<SearchAgentState> {
        self.agent.call(ctx, state).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Debug for AgentNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug_node(f)
    }
}

struct ToolsNode {
    agent: Arc<SearchAgent>,
    name: String,
}

#[async_trait]
impl Node<SearchAgentState> for ToolsNode {
    async fn process(
        &self,
        ctx: &Context,
        state: SearchAgentState,
    ) -> NodeResult<SearchAgentState> {
        self.agent.execute_tools(ctx, state).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Debug for ToolsNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug_node(f)
    }
}

#[derive(Clone)]
pub struct SearchAgent {
    client: Arc<ChatClientImpl>,
    search_tool: Arc<SearchToolsWebSearch>,
    options: ChatCompletionRequestOptions,
}

impl SearchAgent {
    pub fn new(openai_api_key: String, langsmith_api_key: String) -> Self {
        let search_tool = SearchToolsWebSearch(SearchTools);
        let search_tool_schema = <SearchToolsWebSearch as ToolFunction>::get_schema();
        println!(
            "Search tool schema: {}",
            serde_json::to_string_pretty(&search_tool_schema).unwrap()
        );
        Self {
            client: Arc::new(
                ChatClientImpl::new(openai_api_key)
                    .with_tracer(Arc::new(LangSmithTracer::new(langsmith_api_key))),
            ),
            search_tool: Arc::new(search_tool),
            options: ChatCompletionRequestOptions {
                model: "gpt-4o-mini".to_string(),
                temperature: Some(0.0),
                tools: Some(vec![search_tool_schema]),
                tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
            },
        }
    }

    async fn call(
        self: &Self,
        ctx: &Context,
        state: SearchAgentState,
    ) -> NodeResult<SearchAgentState> {
        let request = self
            .client
            .create_chat_completion_request(state.messages.clone(), &self.options)
            .map_err(|e| NodeError::Execution(e.to_string()))?;
        let response = self
            .client
            .complete(
                request,
                Some(ChatCompletionCallOptions::new(
                    Some(ctx.trace_id.clone()),
                    ctx.parent_trace_id.clone(),
                )),
            )
            .await
            .map_err(|e| NodeError::Execution(e.to_string()))?;
        let mut new_messages = vec![];
        match response.choices.first() {
            Some(choice) => {
                let content = match &choice.message.content {
                    Some(content) => content,
                    _ => "",
                };
                let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
                let assistant_message = ChatCompletionRequestAssistantMessageArgs::default()
                    .content(content)
                    .tool_calls(tool_calls)
                    .build()?
                    .into();
                new_messages.push(assistant_message);
            }
            None => {
                return Err(NodeError::Execution("No response choices".to_string()));
            }
        }
        let updates = vec![SearchAgentStateUpdate::Messages(new_messages)];
        Ok(NodeOutput::Updates(updates))
    }

    async fn execute_tools(
        self: &Self,
        _ctx: &Context,
        state: SearchAgentState,
    ) -> NodeResult<SearchAgentState> {
        let mut new_messages = vec![];
        let messages = state.get_latest_messages(1);
        let last_message = messages.first().unwrap();
        match last_message {
            ChatCompletionRequestMessage::Assistant(asst_msg) => {
                let search_tool_name = <SearchToolsWebSearch as ToolFunction>::name();
                let tool_calls = asst_msg.tool_calls.clone().unwrap_or_default();
                for tool_call in tool_calls {
                    match tool_call.function.name.as_str() {
                        name if name == search_tool_name => {
                            let params: SearchParams =
                                serde_json::from_str(&tool_call.function.arguments)
                                    .map_err(|e| NodeError::Execution(e.to_string()))?;
                            let search_results = self.search_tool.execute(params).await?;
                            let tool_response = ChatCompletionRequestToolMessageArgs::default()
                                .content(json!(search_results).to_string())
                                .tool_call_id(tool_call.id)
                                .build()?
                                .into();
                            new_messages.push(tool_response);
                        }
                        _ => return Err(NodeError::Execution("Unknown tool call".to_string())),
                    }
                }
            }
            _ => {
                return Err(NodeError::Execution(
                    "No assistant message found".to_string(),
                ));
            }
        }
        let updates = vec![SearchAgentStateUpdate::Messages(new_messages)];
        Ok(NodeOutput::Updates(updates))
    }

    pub fn build_graph(self: &Self) -> Graph<SearchAgentState, Built> {
        let mut graph = Graph::new("search_agent");

        let call_agent_node = AgentNode {
            agent: Arc::new(self.clone()),
            name: "agent".to_string(),
        };
        let call_tools_node = ToolsNode {
            agent: Arc::new(self.clone()),
            name: "tools".to_string(),
        };
        graph.add_node(call_agent_node);
        graph.add_node(call_tools_node);
        graph.add_edge(START, "agent");
        graph.add_conditional_edge("agent", |state: &SearchAgentState| {
            if state.latest_message_has_tool_calls() {
                "tools".to_string()
            } else {
                END.to_string()
            }
        });
        graph.add_edge("tools", "agent");

        let built_graph = graph.build();
        built_graph
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let openai_api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let langsmith_api_key =
        std::env::var("LANGSMITH_API_KEY").expect("LANGSMITH_API_KEY must be set");

    let context = Context::default();
    let agent = SearchAgent::new(openai_api_key, langsmith_api_key);
    let initial_state = SearchAgentState::new(
        Some("You are a search agent that uses search tools to answer user queries.".to_string()),
        Some("Tell me about Rust's latest release".to_string()),
    );
    let result = agent.build_graph().run(&context, initial_state).await?;
    result.get_latest_messages(2).iter().for_each(|msg| {
        println!("{:?}", msg);
    });
    Ok(())
}
