mod error;
mod message;

// Re-export from error.rs
pub use error::{Error, Result};

// Re-export from message.rs - these need to match the exact types defined in message.rs
pub use message::{
    Message,
    MessageType,
    MessagesState,
    ToolCall,
};