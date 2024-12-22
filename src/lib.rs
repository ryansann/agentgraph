//! AgentGraph is a framework for building stateful, multi-actor applications with LLMs.

pub mod completion;
pub mod graph;
pub mod types;

pub mod prelude {
    //! Convenient re-exports of commonly used types
    pub use crate::completion::{
        ChatClient, 
        ChatClientImpl,
        ChatCompletionRequestOptions,
        ChatCompletionCallOptions,
        LangSmithTracer, 
        TracingProvider, 
        TracingError
    };
    pub use crate::graph::{
        Graph, 
        Node, 
        Context, 
        FunctionNode, 
        START, 
        END,
    };
    pub use crate::types::{
        Result, 
        Error
    };
}

// Re-export main types
pub use prelude::*;