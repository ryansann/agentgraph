use crate::types::{GraphError, GraphResult};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

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
