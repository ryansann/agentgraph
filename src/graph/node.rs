use async_trait::async_trait;
use std::fmt::Debug;
use crate::types::Result;

/// Context for node execution
#[derive(Debug, Clone)]
pub struct Context {
    /// Unique identifier for tracing
    pub trace_id: String,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl Context {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Core trait for graph nodes
#[async_trait]
pub trait Node<State>: Send + Sync + Debug {
    /// Process the current state and return an updated state
    async fn process(&self, ctx: &Context, state: State) -> Result<State>;
    
    /// Get the name of this node
    fn name(&self) -> &str;
}

/// A node that processes state using a function
#[derive(Debug)]
pub struct FunctionNode<State, F> {
    name: String,
    f: F,
    _phantom: std::marker::PhantomData<State>,
}

impl<State, F, Fut> FunctionNode<State, F>
where
    F: Fn(&Context, State) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<State>> + Send,
{
    pub fn new(name: impl Into<String>, f: F) -> Self {
        Self {
            name: name.into(),
            f,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<State, F, Fut> Node<State> for FunctionNode<State, F>
where
    State: Send,
    F: Fn(&Context, State) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<State>> + Send,
{
    async fn process(&self, ctx: &Context, state: State) -> Result<State> {
        (self.f)(ctx, state).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_function_node() {
        let node = FunctionNode::new("test", |_ctx, state: i32| async move { Ok(state + 1) });
        
        let ctx = Context::new("test");
        let result = node.process(&ctx, 1).await.unwrap();
        
        assert_eq!(result, 2);
        assert_eq!(node.name(), "test");
    }
}