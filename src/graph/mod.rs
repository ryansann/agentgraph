mod node;
mod state;

use std::collections::HashMap;
use std::sync::Arc;
use std::fmt::Debug;
use async_trait::async_trait;

use crate::types::{Error, Result};

pub use node::{Context, Node, FunctionNode};
pub use state::{Built, NotBuilt, Edge, NodeConfig, GraphState};

pub const START: &str = "_START_";
pub const END: &str = "_END_";

/// A graph that executes nodes in a defined order
#[derive(Debug)]
pub struct Graph<State, BuildState = NotBuilt> {
    graph_name: String,
    nodes: HashMap<String, Arc<dyn Node<State>>>,
    edges: HashMap<String, Edge<State>>,
    configs: HashMap<String, NodeConfig>,
    _build_state: std::marker::PhantomData<BuildState>,
}

impl<State> Graph<State, NotBuilt>
where
    State: Send + Sync + 'static,
{
    /// Create a new graph
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            graph_name: name.into(),
            nodes: HashMap::new(),
            edges: HashMap::new(),
            configs: HashMap::new(),
            _build_state: std::marker::PhantomData,
        }
    }

    /// Add a node to the graph
    pub fn add_node<N>(&mut self, node: N) -> &mut Self
    where
        N: Node<State> + 'static,
    {
        self.nodes.insert(node.name().to_string(), Arc::new(node));
        self
    }

    /// Add a direct edge between nodes
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        self.edges.insert(from.into(), Edge::Direct(to.into()));
        self
    }

    /// Add a conditional edge from a node
    pub fn add_conditional_edge<F>(&mut self, from: impl Into<String>, condition: F) -> &mut Self
    where
        F: Fn(&State) -> String + Send + Sync + 'static,
    {
        self.edges.insert(from.into(), Edge::Conditional(Arc::new(condition)));
        self
    }

    /// Configure a node with specific settings
    pub fn configure_node(&mut self, name: impl Into<String>, config: NodeConfig) -> &mut Self {
        self.configs.insert(name.into(), config);
        self
    }

    /// Build the graph, making it ready for execution
    pub fn build(self) -> Graph<State, Built> {
        // Validate graph structure here
        // For now, we just transform the state
        Graph {
            graph_name: self.graph_name,
            nodes: self.nodes,
            edges: self.edges,
            configs: self.configs,
            _build_state: std::marker::PhantomData,
        }
    }
}

impl<State> Graph<State, Built>
where
    State: Clone + Send + Sync + 'static,
{
    /// Run the graph with an initial state
    pub async fn run(&self, ctx: &Context, initial_state: State) -> Result<State> {
        let mut current_state = initial_state;
        let mut current_node = START.to_string();

        while current_node != END {
            // Get next node based on edges
            let next_node = match self.edges.get(&current_node) {
                Some(Edge::Direct(next)) => next.clone(),
                Some(Edge::Conditional(condition)) => condition(&current_state),
                None => {
                    if current_node == START {
                        // If we're at START with no edge, try to find a default starting node
                        self.nodes.keys().next()
                            .ok_or_else(|| Error::InvalidState("Graph has no nodes".into()))?
                            .clone()
                    } else {
                        return Err(Error::InvalidTransition(format!(
                            "No transition defined from node: {}", current_node
                        )));
                    }
                }
            };

            // Check if we've reached the end
            if next_node == END {
                break;
            }

            // Get and execute the next node
            let node = self.nodes.get(&next_node)
                .ok_or_else(|| Error::NodeNotFound(next_node.clone()))?;

            // Get node config if it exists, or use default
            let config = self.configs.get(&next_node)
                .cloned()
                .unwrap_or_default();

            // Execute node with retry logic
            let mut attempts = 0;
            let result = loop {
                attempts += 1;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(config.timeout),
                    node.process(ctx, current_state.clone())
                ).await {
                    Ok(Ok(new_state)) => break Ok(new_state),
                    Ok(Err(_e)) if attempts < config.max_retries => {
                        current_state = current_state;
                        continue;
                    }
                    Ok(Err(e)) => break Err(e),
                    Err(_) if attempts < config.max_retries => {
                        current_state = current_state;
                        continue;
                    }
                    Err(_) => break Err(Error::ExecutionError(
                        format!("Node {} timed out after {} attempts", next_node, attempts)
                    )),
                }
            }?;

            current_state = result;
            current_node = next_node;
        }

        Ok(current_state)
    }
}

#[async_trait]
impl<State> Node<State> for Graph<State, Built>
where
    State: Clone + Send + Sync + Debug + 'static,
{
    async fn process(&self, ctx: &Context, state: State) -> Result<State> {
        // Simply delegate to the run method
        self.run(ctx, state).await
    }

    fn name(&self) -> &str {
        // Each graph should have a name for debugging and tracing
        &self.graph_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use node::FunctionNode;

    #[tokio::test]
    async fn test_basic_graph() {
        // Create nodes
        let node1 = FunctionNode::new("node1", |_ctx, state: i32| async move { Ok(state + 1) });
        let node2 = FunctionNode::new("node2", |_ctx, state: i32| async move { Ok(state * 2) });

        // Build graph
        let built_graph = {
            let mut graph = Graph::new("g");
            graph.add_node(node1)
                .add_node(node2)
                .add_edge("node1", "node2")
                .add_edge(START, "node1")
                .add_edge("node2", END);
            graph.build()
        };

        // Run graph
        let ctx = Context::new("test");
        let result = built_graph.run(&ctx, 1).await.unwrap();

        // 1 + 1 = 2, 2 * 2 = 4
        assert_eq!(result, 4);
    }

    #[tokio::test]
    async fn test_conditional_graph() {
        // Create nodes
        let node1 = FunctionNode::new("node1", |_ctx, state: i32| async move { Ok(state + 1) });
        let node2 = FunctionNode::new("node2", |_ctx, state: i32| async move { Ok(state * 2) });

        // Build graph with condition
        let built_graph = {
            let mut graph = Graph::new("g");
            graph.add_node(node1)
                .add_node(node2)
                .add_edge(START, "node1")
                .add_edge("node2", END)
                .add_conditional_edge("node1", |state: &i32| {
                    if *state < 5 { "node2".into() } else { END.into() }
                });
            graph.build()
        };

        // Test when condition routes to node2
        let ctx = Context::new("test1");
        let result = built_graph.run(&ctx, 1).await.unwrap();
        assert_eq!(result, 4);

        // Test when condition routes to END
        let ctx = Context::new("test2");
        let result = built_graph.run(&ctx, 5).await.unwrap();
        assert_eq!(result, 6);
    }
}