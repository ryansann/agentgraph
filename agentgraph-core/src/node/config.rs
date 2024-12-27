/// Configuration for node execution
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Maximum retries for node execution
    pub max_retries: usize,
    /// Timeout for node execution in seconds
    pub timeout: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout: 30,
        }
    }
}

/// Builder for node configuration
pub struct NodeConfigBuilder {
    config: NodeConfig,
}

impl NodeConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: NodeConfig::default(),
        }
    }

    pub fn max_retries(mut self, retries: usize) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }

    pub fn build(self) -> NodeConfig {
        self.config
    }
}
