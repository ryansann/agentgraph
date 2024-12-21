// src/completion/mod.rs

mod client;
mod tracing;

pub use client::{ChatClient, ChatClientImpl, TracingProvider};
pub use tracing::{};
