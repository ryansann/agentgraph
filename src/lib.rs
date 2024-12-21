//! AgentGraph is a framework for building stateful, multi-actor applications with LLMs.

pub mod graph;
pub mod types;
pub mod utils;

pub mod prelude {
    //! Convenient re-exports of commonly used types
    //pub use crate::agent::{BaseAgent, Tool, LLM};
    pub use crate::graph::{Graph, Node, Context, FunctionNode, START, END};
    pub use crate::types::{Message, MessageType, Result, Error, MessagesState};
}

// Re-export main types
pub use prelude::*;