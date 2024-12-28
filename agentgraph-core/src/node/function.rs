use crate::node::Context;
use crate::node::Node;
use crate::types::GraphResult;
use crate::types::{GraphState, NodeResult};
use async_trait::async_trait;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result;
use std::future::Future;

/// A node that processes state using a function
pub struct FunctionNode<S, F> {
    name: String,
    f: F,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, F, Fut> FunctionNode<S, F>
where
    S: Debug + Send + Sync + GraphState,
    F: Fn(&Context, S) -> Fut + Send + Sync,
    Fut: Future<Output = GraphResult<S>> + Send,
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
impl<S, F, Fut> Node<S> for FunctionNode<S, F>
where
    S: Debug + Send + Sync + GraphState,
    F: Fn(&Context, S) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = NodeResult<S>> + Send,
{
    async fn process(&self, ctx: &Context, state: S) -> NodeResult<S> {
        (self.f)(ctx, state).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Instead of #[derive(Debug)]
impl<S, F> Debug for FunctionNode<S, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("FunctionNode")
            .field("name", &self.name)
            // Skip the function field since it can't implement Debug
            .finish()
    }
}
