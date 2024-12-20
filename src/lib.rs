//! Graphite is a framework for building stateful, multi-actor applications with LLMs.
//! 
//! # Examples
//! 
//! ```rust,no_run
//! use graphite::prelude::*;
//! 
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let agent = BaseAgent::new(/* ... */);
//! # Ok(())
//! # }
//! ```

pub mod agent;
pub mod graph;
pub mod types;
pub mod utils;

pub mod prelude {
    //! Convenient re-exports of commonly used types
    //pub use crate::agent::{BaseAgent, Tool, LLM};
    pub use crate::graph::{Graph, Node};
    pub use crate::types::{Message, MessageType, Result, Error};
}

// Re-export main types
pub use prelude::*;