use super::context::Context;
use crate::types::GraphResult;
use async_trait::async_trait;
use std::fmt::Debug;

/// Core trait for graph nodes
#[async_trait]
pub trait Node<State>: Send + Sync + Debug {
    /// Process the current state and return an updated state
    async fn process(&self, ctx: &Context, state: State) -> GraphResult<State>;

    /// Get the name of this node
    fn name(&self) -> &str;
}
