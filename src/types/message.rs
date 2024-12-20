use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents different types of messages in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Human,
    AI,
    System,
    Tool,
}

/// Represents a tool call with its arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: HashMap<String, serde_json::Value>,
}

/// Represents a message in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_type: MessageType,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<HashMap<String, serde_json::Value>>,
}

impl Message {
    /// Creates a new human message
    pub fn human(content: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Human,
            content: content.into(),
            tool_calls: None,
            status: None,
            additional: None,
        }
    }

    /// Creates a new AI message
    pub fn ai(content: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::AI,
            content: content.into(),
            tool_calls: None,
            status: None,
            additional: None,
        }
    }

    /// Creates a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::System,
            content: content.into(),
            tool_calls: None,
            status: None,
            additional: None,
        }
    }

    /// Creates a new tool message
    pub fn tool(content: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Tool,
            content: content.into(),
            tool_calls: None,
            status: Some(status.into()),
            additional: None,
        }
    }

    /// Adds tool calls to the message
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    /// Adds additional data to the message
    pub fn with_additional(mut self, additional: HashMap<String, serde_json::Value>) -> Self {
        self.additional = Some(additional);
        self
    }
}

/// Represents the state of messages in a conversation
#[derive(Debug, Clone, Default)]
pub struct MessagesState {
    pub messages: Vec<Message>,
    pub errors: Vec<String>,
}

impl MessagesState {
    /// Creates a new empty message state
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a message to the state
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Adds an error to the state
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Gets the last message in the state
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Gets the number of errors
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Merges another state into this one
    pub fn merge(mut self, other: Self) -> Self {
        self.messages.extend(other.messages);
        self.errors.extend(other.errors);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let human_msg = Message::human("Hello");
        assert_eq!(human_msg.message_type, MessageType::Human);

        let ai_msg = Message::ai("Hi there");
        assert_eq!(ai_msg.message_type, MessageType::AI);

        let system_msg = Message::system("System prompt");
        assert_eq!(system_msg.message_type, MessageType::System);

        let tool_msg = Message::tool("Result", "success");
        assert_eq!(tool_msg.message_type, MessageType::Tool);
        assert_eq!(tool_msg.status, Some("success".to_string()));
    }

    #[test]
    fn test_message_state() {
        let mut state = MessagesState::new();
        assert_eq!(state.messages.len(), 0);
        assert_eq!(state.errors.len(), 0);

        state.add_message(Message::human("Hello"));
        assert_eq!(state.messages.len(), 1);

        state.add_error("Test error");
        assert_eq!(state.error_count(), 1);

        let last_msg = state.last_message().unwrap();
        assert_eq!(last_msg.message_type, MessageType::Human);
    }
}