use super::context::Context;
use crate::node::Node;
use crate::types::GraphResult;
use async_trait::async_trait;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result;

/// A node that processes state using a function
pub struct FunctionNode<State, F> {
    name: String,
    f: F,
    _phantom: std::marker::PhantomData<State>,
}

// Instead of #[derive(Debug)]
impl<State, F> Debug for FunctionNode<State, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("FunctionNode")
            .field("name", &self.name)
            // Skip the function field since it can't implement Debug
            .finish()
    }
}

impl<State, F, Fut> FunctionNode<State, F>
where
    State: Debug + Send,
    F: Fn(&Context, State) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = GraphResult<State>> + Send,
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
    State: Debug + Send + Sync,
    F: Fn(&Context, State) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = GraphResult<State>> + Send,
{
    async fn process(&self, ctx: &Context, state: State) -> GraphResult<State> {
        (self.f)(ctx, state).await
    }

    fn name(&self) -> &str {
        &self.name
    }
}
