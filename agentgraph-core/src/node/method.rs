use crate::types::{GraphState, NodeResult};
use crate::node::{Node, Context};
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::fmt::{Result, Formatter, Debug};

// First, create a wrapper struct for method nodes
pub struct MethodNode<T, S: GraphState> {
    name: String,
    instance: T,
    method: fn(&T, &Context, S) -> Pin<Box<dyn Future<Output = NodeResult<S>> + Send>>,
}

impl<T, S> MethodNode<T, S> 
where 
    T: Send + Sync + 'static,
    S: GraphState,
{
    pub fn new(
        name: impl Into<String>,
        instance: T,
        method: fn(&T, &Context, S) -> Pin<Box<dyn Future<Output = NodeResult<S>> + Send>>,
    ) -> Self {
        Self {
            name: name.into(),
            instance,
            method,
        }
    }
}

// Implement Node trait for MethodNode
#[async_trait]
impl<T, S> Node<S> for MethodNode<T, S>
where
    T: Send + Sync + 'static,
    S: GraphState,
{
    async fn process(&self, ctx: &Context, state: S) -> NodeResult<S> {
        (self.method)(&self.instance, ctx, state).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl <T, S: GraphState> Debug for MethodNode<T, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("MethodNode")
            .field("name", &self.name)
            .finish()
    }
}