use std::sync::Arc;

/// Represents a condition for edge transitions
pub type Condition<S> = Arc<dyn Fn(&S) -> String + Send + Sync>;

/// Edge definition for graph transitions
#[derive(Clone)]
pub enum Edge<S> {
    /// Direct edge to next node
    Direct(String),
    /// Conditional edge based on state
    Conditional(Condition<S>),
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
