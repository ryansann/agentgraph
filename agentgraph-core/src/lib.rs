//! AgentGraph is a framework for building stateful, multi-actor applications with LLMs.

#![allow(unused_extern_crates)]
extern crate self as agentgraph_core;

pub mod completion;
pub mod graph;
pub mod node;
pub mod tool;
pub mod types;

pub mod prelude {
    //! Convenient re-exports of commonly used types
    pub use crate::completion::{
        ChatClient, ChatClientImpl, ChatCompletionCallOptions, ChatCompletionRequestOptions,
        LangSmithTracer, TracingError, TracingProvider,
    };
    pub use crate::graph::{Graph, END, START};
    pub use crate::node::{Context, FunctionNode, Node};
    pub use crate::tool::{JsonSchema, ToolError, ToolFunction};
    pub use crate::types::{GraphError, GraphResult, GraphState, UpdateableState};
}

// Re-export main types
pub use prelude::*;
