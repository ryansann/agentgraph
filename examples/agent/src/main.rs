use agentgraph_core::prelude::*;
use agentgraph_macros::{State, tools};
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    ChatCompletionToolChoiceOption, ChatCompletionRequestMessage,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use async_trait::async_trait;
use std::fmt::{Debug, Formatter};

// Search tool types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchParams {
    #[schemars(description = "The search query to execute")]
    query: String,

    #[schemars(description = "Maximum number of results to return (default: 5)")]
    max_results: Option<usize>,
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

        let max_results = params.max_results.unwrap_or(5);
        let results = vec![result; max_results];

        Ok(SearchResponse { results })
    }
}

#[derive(State, Debug, Clone, Serialize, Deserialize)]
pub struct SearchAgentState {
    messages: Vec<ChatCompletionRequestMessage>,
    errors: Vec<String>,
}

impl SearchAgentState {
    pub fn new(system: Option<String>, user : Option<String>) -> Self {
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
                    asst_msg.tool_calls.as_ref().map_or(false, |calls| !calls.is_empty())
                }
                // Other message types (System, User) can't have tool calls
                _ => false
            }
        })
    }

    pub fn get_latest_messages(&self, count: usize) -> Vec<ChatCompletionRequestMessage> {
        let mut result = Vec::new();

        // Include the first message if it's a system message
        if let Some(first) = self.messages.first() {
            if matches!(first, ChatCompletionRequestMessage::System(_)) {
                result.push(first.clone());
            }
        }

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
    async fn process(&self, ctx: &Context, state: SearchAgentState) -> NodeResult<SearchAgentState> {
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
    async fn process(&self, ctx: &Context, state: SearchAgentState) -> NodeResult<SearchAgentState> {
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

    pub fn new(openai_api_key: String) -> Self {
        let search_tool = SearchToolsWebSearch(SearchTools);
        let search_tool_schema = <SearchToolsWebSearch as ToolFunction>::get_schema();
        Self {
            client: Arc::new(ChatClientImpl::new(openai_api_key)),
            search_tool: Arc::new(search_tool),
            options: ChatCompletionRequestOptions {
                model: "gpt-4o-mini".to_string(),
                temperature: Some(0.0),
                tools: Some(vec![search_tool_schema]),
                tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
            },
        }
    }

    async fn call(self: &Self, ctx: &Context, state: SearchAgentState) -> NodeResult<SearchAgentState> {
        let request = self
            .client
            .create_chat_completion_request(state.messages.clone(), self.options)?;
        let response = self
            .client
            .complete(request, Some(ChatCompletionCallOptions::new(Some(ctx.trace_id), None)))
            .await?;

    // For example, take the first choice's message and push it into `state.messages`.
    let new_message = ChatCompletionRequestMessage::Assistant(response.choices[0].message.clone());
    state.messages.push(new_message);

    // Now return NodeOutput::Full(updated_state)
    Ok(NodeOutput::Full(state))
    }

    async fn execute_tools(self: &Self, ctx: &Context, state: SearchAgentState) -> NodeResult<SearchAgentState> {
        todo!();
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

        let built_graph = graph.build();
        built_graph
    }

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let openai_api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    let context = Context::default();
    let agent = SearchAgent::new(openai_api_key);
    let initial_state = SearchAgentState::new(
        Some("You are a search agent that uses tools to answer user queries".to_string()), 
        Some("Tell me about lifetimes in Rust".to_string()),
    );
    let result = agent.build_graph().run(&context, initial_state).await?;
    result.get_latest_messages(2).iter().for_each(|msg| {
        println!("{:?}", msg);
    });
    Ok(())
}
