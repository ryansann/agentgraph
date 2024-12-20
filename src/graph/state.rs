use std::fmt::Debug;
use crate::types::{Error, Result};

/// Marker types for graph construction states
pub struct NotBuilt;
pub struct Built;

/// Represents a condition for edge transitions
pub type Condition<State> = Box<dyn Fn(&State) -> String + Send + Sync>;

/// Edge definition for graph transitions
#[derive(Debug)]
pub enum Edge<State> {
    /// Direct edge to next node
    Direct(String),
    /// Conditional edge based on state
    Conditional(Condition<State>),
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

/// Trait for stateful objects that can be cloned
pub trait StateClone: Send + Sync {
    fn clone_box(&self) -> Box<dyn StateClone>;
}

impl<T> StateClone for T
where
    T: 'static + Send + Sync + Clone,
{
    fn clone_box(&self) -> Box<dyn StateClone> {
        Box::new(self.clone())
    }
}

/// Trait for graph state management
pub trait GraphState: StateClone + Debug {
    /// Merge this state with another state
    fn merge(&mut self, other: Box<dyn GraphState>) -> Result<()>;
}

impl Clone for Box<dyn GraphState> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestState {
        value: i32,
    }

    impl GraphState for TestState {
        fn merge(&mut self, other: Box<dyn GraphState>) -> Result<()> {
            if let Some(other) = other.as_any().downcast_ref::<TestState>() {
                self.value += other.value;
                Ok(())
            } else {
                Err(Error::InvalidState("Cannot merge different state types".into()))
            }
        }
    }

    #[test]
    fn test_node_config() {
        let config = NodeConfigBuilder::new()
            .max_retries(5)
            .timeout(60)
            .build();
        
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.timeout, 60);
    }
}