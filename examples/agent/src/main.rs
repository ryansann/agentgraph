use agentgraph_core::prelude::*;
use agentgraph_macros::tools;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    ChatCompletionToolChoiceOption,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

pub struct SearchAgentState {
    messages: Vec<ChatCompletionRequestMessage>,
    errors: Vec<String>,
}

impl SearchAgentState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn get_latest_messages(&self) -> Vec<ChatCompletionRequestMessage> {
        let mut result = Vec::new();

        // Add system message if it exists as the first message
        if let Some(first) = self.messages.first() {
            if matches!(first, ChatCompletionRequestMessage::System(_)) {
                result.push(first.clone());
            }
        }

        // Add up to 4 most recent non-system messages
        result.extend(
            self.messages
                .iter()
                .rev() // Reverse to get most recent first
                .filter(|msg| !matches!(msg, ChatCompletionRequestMessage::System(_))) // Skip system messages
                .take(4) // Take up to 4 messages
                .rev() // Reverse back to maintain chronological order
                .cloned(),
        );

        result
    }

}

pub struct SearchAgent {
    state: SearchAgentState,
    client: ChatClientImpl,
    search_tool: SearchToolsWebSearch,
}

impl SearchAgent {
    pub fn new(openai_api_key: String) -> Self {
        Self {
            state: SearchAgentState::new(),
            client: ChatClientImpl::new(openai_api_key),
            search_tool: SearchToolsWebSearch(SearchTools),
        }
    }

    pub fn build_graph() -> Graph {
        let mut graph = Graph::new();

        // Add nodes for the search workflow
        graph.add_node("input", NodeType::Input);
        graph.add_node(
            "search",
            NodeType::Tool(Box::new(SearchToolsWebSearch(SearchTools))),
        );
        graph.add_node("process_results", NodeType::LLM);
        graph.add_node("output", NodeType::Output);

        // Connect the nodes
        graph.connect("input", "search").unwrap();
        graph.connect("search", "process_results").unwrap();
        graph.connect("process_results", "output").unwrap();

        graph
    }

}

impl Node<SearchAgentState> for SearchAgent {
    async fn process(
        &self,
        ctx: &Context,
        state: SearchAgentState,
    ) -> GraphResult<SearchAgentState> {
        let search_schema = <SearchToolsWebSearch as ToolFunction>::get_schema();
        let tools = vec![search_schema];

        // Create request options
        let options = ChatCompletionRequestOptions {
            model: "gpt-4".to_string(),
            temperature: Some(0.0),
            tools: Some(tools),
            tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
        };

        // Create and send the request
        let request = self
            .client
            .create_chat_completion_request(messages, options)?;
        let response = self.client.complete(request, None).await?;

        // Process the response and tool calls
        for choice in response.choices {
            println!(
                "\nAssistant: {}",
                choice.message.content.unwrap_or_default()
            );

            // Handle any tool calls
            if let Some(tool_calls) = choice.message.tool_calls {
                let search_name = <SearchToolsWebSearch as ToolFunction>::name();

                for tool_call in tool_calls {
                    if tool_call.function.name == search_name {
                        let params: SearchParams =
                            serde_json::from_str(&tool_call.function.arguments)?;
                        let search_results =
                            ToolFunction::execute(&self.search_tool, params).await?;

                        println!("\nSearch Results:");
                        for result in search_results.results {
                            println!("Title: {}", result.title);
                            println!("URL: {}", result.url);
                            println!("Snippet: {}", result.snippet);
                            println!();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let openai_api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    let agent = SearchAgent::new(openai_api_key);
    agent
        .execute_search("What is Rust programming language?")
        .await?;

    Ok(())
}
