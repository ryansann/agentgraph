/// Context for node execution
#[derive(Debug, Clone)]
pub struct Context {
    /// Parent trace identifier
    pub parent_trace_id: Option<String>,
    /// Unique identifier for tracing
    pub trace_id: String,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl Context {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            parent_trace_id: None,
            trace_id: trace_id.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_parent_trace_id(mut self, parent_trace_id: impl Into<String>) -> Self {
        self.parent_trace_id = Some(parent_trace_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
