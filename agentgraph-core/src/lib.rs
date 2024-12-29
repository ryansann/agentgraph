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
    pub use crate::graph::{Condition, Edge, Graph, END, START, Built, NotBuilt};
    pub use crate::node::{Context, FunctionNode, MethodNode, Node};
    pub use crate::tool::{JsonSchema, ToolFunction};
    pub use crate::types::{
        GraphError, GraphResult, GraphState, NodeError, NodeOutput, NodeResult, ToolError,
    };
}

// Re-export main types
pub use prelude::*;
