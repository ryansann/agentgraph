use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use super::*;
use crate::node::*;
use crate::types::*;

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

impl<S> Graph<S, NotBuilt>
where
    S: Send + Sync + 'static + Clone + Debug + GraphState,
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
        N: Node<S> + 'static,
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
        F: Fn(&S) -> String + Send + Sync + 'static,
    {
        self.edges
            .insert(from.into(), Edge::Conditional(Arc::new(condition)));
        self
    }

    /// Configure a node with specific settings
    pub fn configure_node(&mut self, name: impl Into<String>, config: NodeConfig) -> &mut Self {
        self.configs.insert(name.into(), config);
        self
    }

    /// Build the graph, making it ready for execution
    pub fn build(self) -> Graph<S, Built> {
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

impl<S> Graph<S, Built>
where
    S: Clone + Send + Sync + 'static + GraphState + Debug,
{
    /// Run the graph with an initial state
    pub async fn run(&self, ctx: &Context, initial_state: S) -> GraphResult<S> {
        let mut current_state = initial_state;
        let mut current_node = START.to_string();

        while current_node != END {
            // Get next node based on edges
            let next_node = match self.edges.get(&current_node) {
                Some(Edge::Direct(next)) => next.clone(),
                Some(Edge::Conditional(condition)) => {
                    let current_state_ref = &current_state;
                    condition(current_state_ref)
                }
                None => {
                    if current_node == START {
                        // If we're at START with no edge, try to find a default starting node
                        self.nodes
                            .keys()
                            .next()
                            .ok_or_else(|| GraphError::InvalidState("Graph has no nodes".into()))?
                            .clone()
                    } else {
                        return Err(GraphError::InvalidTransition(format!(
                            "No transition defined from node: {}",
                            current_node
                        )));
                    }
                }
            };

            // Check if we've reached the end
            if next_node == END {
                break;
            }

            // Get and execute the next node
            let node = self
                .nodes
                .get(&next_node)
                .ok_or_else(|| GraphError::NodeNotFound(next_node.clone()))?;

            // Get node config if it exists, or use default
            let config = self.configs.get(&next_node).cloned().unwrap_or_default();

            // Execute node with retry logic
            let mut node_ctx = ctx.clone();
            let mut attempts = 0;
            let updates = loop {
                attempts += 1;
                if attempts > 1 {
                    node_ctx = node_ctx.next_node_context();
                }
                // Node::process returns NodeResult<S::Update> = Result<Vec<S::Update>, NodeError>
                match tokio::time::timeout(
                    std::time::Duration::from_secs(config.timeout),
                    node.process(&node_ctx, current_state.clone()),
                )
                .await
                {
                    // (Ok(Ok(updates))) => success from Node
                    Ok(Ok(updates)) => break Ok(updates),

                    // (Ok(Err(e))) => Node returned an error
                    Ok(Err(e)) if attempts < config.max_retries => {
                        // optionally do something like logging the error
                        // we don't modify `current_state` yet, so just retry
                        continue;
                    }
                    Ok(Err(e)) => {
                        break Err(e); // bubble up NodeError
                    }

                    // (Err(_)) => timed out waiting for node
                    Err(_) if attempts < config.max_retries => {
                        // optionally log the timeout
                        continue;
                    }
                    Err(_) => {
                        break Err(NodeError::Execution(format!(
                            "Node {} timed out after {} attempts",
                            next_node, attempts
                        )))
                    }
                }
            }?;

            // Now apply each update to our state
            current_state = match updates {
                NodeOutput::Full(new_state) => new_state,
                NodeOutput::Updates(updates) => {
                    let mut new_state = current_state.clone();
                    new_state.apply_many(updates);
                    new_state
                }
            };

            // Move on
            current_node = next_node;
        }

        Ok(current_state)
    }
}

#[async_trait]
impl<S> Node<S> for Graph<S, Built>
where
    S: Clone + Send + Sync + Debug + 'static + GraphState,
{
    async fn process(&self, ctx: &Context, state: S) -> NodeResult<S> {
        let new_state = self
            .run(ctx, state.clone())
            .await
            .map_err(|e| NodeError::SubgraphExecution(e.to_string()))?;
        Ok(NodeOutput::Full(new_state))
    }

    fn name(&self) -> &str {
        &self.graph_name
    }
}
