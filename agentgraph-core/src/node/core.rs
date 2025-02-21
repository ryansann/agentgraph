use crate::{Context, GraphState, NodeResult};
use async_trait::async_trait;
use std::fmt::{Debug, Result, Formatter};

/// Core trait for graph nodes
#[async_trait]
pub trait Node<S>: Send + Sync + Debug
where
    S: GraphState,
{
    /// Process the current state and return a update to the state
    async fn process(&self, ctx: &Context, state: S) -> NodeResult<S>;

    /// Get the name of this node
    fn name(&self) -> &str;

    fn debug_node(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Node({})", self.name())
    }
}
