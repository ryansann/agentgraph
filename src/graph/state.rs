use crate::types::{GraphError, GraphResult};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

/// Marker types for graph construction states
#[derive(Debug)]
pub struct NotBuilt;
#[derive(Debug)]
pub struct Built;

/// Represents a condition for edge transitions
pub type Condition<State> = Arc<dyn Fn(&State) -> String + Send + Sync>;

/// Edge definition for graph transitions
#[derive(Clone)]
pub enum Edge<State> {
    /// Direct edge to next node
    Direct(String),
    /// Conditional edge based on state
    Conditional(Condition<State>),
}

// Manual Debug implementation
impl<State> std::fmt::Debug for Edge<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edge::Direct(target) => f.debug_tuple("Direct").field(target).finish(),
            Edge::Conditional(_) => f
                .debug_tuple("Conditional")
                .field(&"<condition>") // Placeholder for the function
                .finish(),
        }
    }
}

/// Configuration for node execution
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Maximum retries for node execution
    pub max_retries: usize,
    /// Timeout for node execution in seconds
    pub timeout: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout: 30,
        }
    }
}

/// Builder for node configuration
pub struct NodeConfigBuilder {
    config: NodeConfig,
}

impl NodeConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: NodeConfig::default(),
        }
    }

    pub fn max_retries(mut self, retries: usize) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }

    pub fn build(self) -> NodeConfig {
        self.config
    }
}

pub trait GraphState: Debug + Send + Sync + Any {
    fn merge(&mut self, other: Box<dyn GraphState>) -> GraphResult<()>;
    fn clone_box(&self) -> Box<dyn GraphState>;
}

impl<T> GraphState for T
where
    T: 'static + Debug + Send + Sync + Clone,
{
    fn merge(&mut self, _other: Box<dyn GraphState>) -> GraphResult<()> {
        Ok(()) // Default implementation
    }

    fn clone_box(&self) -> Box<dyn GraphState> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test state implementation
    #[derive(Debug, Clone)]
    struct CounterState {
        count: i32,
    }

    // Test edge creation and debug formatting
    #[test]
    fn test_edge_creation_and_debug() {
        // Test Direct edge
        let direct_edge: Edge<CounterState> = Edge::Direct("next".to_string());
        assert!(format!("{:?}", direct_edge).contains("Direct"));

        // Test Conditional edge
        let condition: Condition<CounterState> = Arc::new(|state: &CounterState| {
            if state.count > 5 {
                "high".to_string()
            } else {
                "low".to_string()
            }
        });
        let cond_edge: Edge<CounterState> = Edge::Conditional(condition);
        assert!(format!("{:?}", cond_edge).contains("Conditional"));
    }

    // Test NodeConfig builder
    #[test]
    fn test_node_config_builder() {
        let config = NodeConfigBuilder::new().max_retries(5).timeout(60).build();

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.timeout, 60);
    }

    // Test NodeConfig default
    #[test]
    fn test_node_config_default() {
        let config = NodeConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout, 30);
    }

    // Test GraphState implementation
    #[test]
    fn test_graph_state() {
        let mut state = CounterState { count: 0 };
        let other_state = CounterState { count: 5 };

        // Test cloning through GraphState trait
        let boxed_state: Box<dyn GraphState> = Box::new(state.clone());

        // Test merging
        state.merge(Box::new(other_state)).unwrap();
    }

    // Test conditional edge execution
    #[test]
    fn test_conditional_edge() {
        let condition: Condition<CounterState> = Arc::new(|state: &CounterState| {
            if state.count > 5 {
                "high".to_string()
            } else {
                "low".to_string()
            }
        });

        let state = CounterState { count: 10 };
        assert_eq!(condition(&state), "high");

        let state = CounterState { count: 3 };
        assert_eq!(condition(&state), "low");
    }

    // Test edge cloning
    #[test]
    fn test_edge_cloning() {
        let direct_edge: Edge<CounterState> = Edge::Direct("next".to_string());
        let cloned_direct = direct_edge.clone();

        match cloned_direct {
            Edge::Direct(target) => assert_eq!(target, "next"),
            _ => panic!("Wrong edge type after cloning"),
        }

        let condition: Condition<CounterState> = Arc::new(|_| "test".to_string());
        let cond_edge: Edge<CounterState> = Edge::Conditional(condition);
        let cloned_cond = cond_edge.clone();

        match cloned_cond {
            Edge::Conditional(c) => assert_eq!(c(&CounterState { count: 0 }), "test"),
            _ => panic!("Wrong edge type after cloning"),
        }
    }
}
