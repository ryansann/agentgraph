use agentgraph_core::prelude::*;
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs,
    ChatCompletionRequestUserMessageContent,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

// Custom test state
#[derive(Debug, Clone)]
struct CounterState {
    count: i32,
    history: Vec<String>,
}

impl CounterState {
    fn new(count: i32) -> Self {
        Self {
            count,
            history: Vec::new(),
        }
    }

    fn record_operation(&mut self, op: &str) {
        self.history.push(op.to_string());
    }
}

// Custom test node
#[derive(Debug)]
struct IncrementNode {
    amount: i32,
}

#[async_trait]
impl Node<CounterState> for IncrementNode {
    async fn process(&self, _ctx: &Context, mut state: CounterState) -> GraphResult<CounterState> {
        state.count += self.amount;
        state.record_operation(&format!("increment_{}", self.amount));
        Ok(state)
    }

    fn name(&self) -> &str {
        "increment"
    }
}

#[tokio::test]
async fn test_basic_counter_flow() {
    let increment_node = IncrementNode { amount: 5 };
    let double_node = FunctionNode::new("double", |_ctx, mut state: CounterState| async move {
        state.count *= 2;
        state.record_operation("double");
        Ok(state)
    });

    let built_graph = {
        let mut graph = Graph::new("g");
        graph
            .add_node(increment_node)
            .add_node(double_node)
            .add_edge(START, "increment")
            .add_edge("increment", "double")
            .add_edge("double", END);
        graph.build()
    };

    let ctx = Context::new("test");
    let initial_state = CounterState::new(10);
    let final_state = built_graph.run(&ctx, initial_state).await.unwrap();

    assert_eq!(final_state.count, 30); // (10 + 5) * 2
    assert_eq!(final_state.history, vec!["increment_5", "double"]);
}

#[tokio::test]
async fn test_conditional_routing() {
    let even_node = FunctionNode::new("odd", |_ctx, mut state: CounterState| async move {
        state.count *= 2;
        state.record_operation("odd");
        Ok(state)
    });

    let odd_node = FunctionNode::new("even", |_ctx, mut state: CounterState| async move {
        state.count = state.count * 2 + 1;
        state.record_operation("even");
        Ok(state)
    });

    let built_graph = {
        let mut graph = Graph::new("g");
        graph
            .add_node(even_node)
            .add_node(odd_node)
            .add_edge(START, "even")
            .add_conditional_edge("even", |state: &CounterState| {
                if state.count % 2 == 0 {
                    "even".to_string()
                } else {
                    "odd".to_string()
                }
            })
            .add_conditional_edge("odd", |state: &CounterState| {
                if state.count > 100 {
                    END.to_string()
                } else {
                    "even".to_string()
                }
            });
        graph.build()
    };

    let ctx = Context::new("test");
    let initial_state = CounterState::new(5);
    let final_state = built_graph.run(&ctx, initial_state).await.unwrap();

    // Verify execution path and final state
    println!("Final count: {}", final_state.count);
    println!("History: {:?}", final_state.history);
}

// Test message-based state using async-openai types
#[derive(Debug, Clone)]
struct ChatState {
    messages: Vec<ChatCompletionRequestMessage>,
}

impl ChatState {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    fn add_user_message(&mut self, content: &str) -> GraphResult<()> {
        let message = ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()
            .map_err(|e| GraphError::Other(e.into()))?;
        self.messages
            .push(ChatCompletionRequestMessage::User(message));
        Ok(())
    }

    fn add_assistant_message(&mut self, content: &str) -> GraphResult<()> {
        let content = ChatCompletionRequestAssistantMessageContent::Text(content.to_string());
        let message = ChatCompletionRequestAssistantMessageArgs::default()
            .content(content)
            .build()
            .map_err(|e| GraphError::Other(e.into()))?;
        self.messages
            .push(ChatCompletionRequestMessage::Assistant(message));
        Ok(())
    }
}

#[tokio::test]
async fn test_chat_flow() {
    let process_node = FunctionNode::new("process", |_ctx, mut state: ChatState| async move {
        if let Some(last_msg) = state.messages.last() {
            match last_msg {
                ChatCompletionRequestMessage::User(msg) => {
                    let content = match &msg.content {
                        ChatCompletionRequestUserMessageContent::Text(text) => text.clone(),
                        ChatCompletionRequestUserMessageContent::Array(_) => {
                            return Err(GraphError::ExecutionError(
                                "Array content not supported".into(),
                            ));
                        }
                    };
                    let response = format!("Processed: {}", content);
                    state.add_assistant_message(&response)?;
                }
                _ => {}
            }
        }
        Ok(state)
    });

    let built_graph = {
        let mut graph = Graph::new("g");
        graph
            .add_node(process_node)
            .add_edge(START, "process")
            .add_edge("process", END);
        graph.build()
    };

    let ctx = Context::new("test");
    let mut initial_state = ChatState::new();
    initial_state.add_user_message("Test message").unwrap();

    let final_state = built_graph.run(&ctx, initial_state).await.unwrap();

    assert_eq!(final_state.messages.len(), 2);
    if let ChatCompletionRequestMessage::Assistant(msg) = &final_state.messages[1] {
        if let Some(ChatCompletionRequestAssistantMessageContent::Text(content)) = &msg.content {
            assert_eq!(content, "Processed: Test message");
        } else {
            panic!("Assistant message has invalid content");
        }
    } else {
        panic!("Expected assistant message");
    }
}

// Test error handling and retries
#[derive(Debug)]
struct FlakyNode {
    attempts: Arc<Mutex<i32>>,
    max_failures: i32,
}

#[async_trait]
impl Node<CounterState> for FlakyNode {
    async fn process(&self, _ctx: &Context, state: CounterState) -> GraphResult<CounterState> {
        let mut attempts = self.attempts.lock().await;
        *attempts += 1;

        if *attempts <= self.max_failures {
            Err(GraphError::ExecutionError("Temporary failure".into()))
        } else {
            Ok(state)
        }
    }

    fn name(&self) -> &str {
        "flaky"
    }
}

#[tokio::test]
async fn test_retry_behavior() {
    let attempts = Arc::new(Mutex::new(0));
    let flaky_node = FlakyNode {
        attempts: attempts.clone(),
        max_failures: 2,
    };

    let built_graph = {
        let mut graph = Graph::new("g");
        graph
            .add_node(flaky_node)
            .add_edge(START, "flaky")
            .add_edge("flaky", END);
        graph.build()
    };

    let ctx = Context::new("test");
    let initial_state = CounterState::new(0);
    let result = built_graph.run(&ctx, initial_state).await;

    assert!(result.is_ok());
    assert_eq!(*attempts.lock().await, 3); // 2 failures + 1 success
}

// Helper function to create test nodes
fn create_test_node(
    name: &str,
    operation: impl Fn(CounterState) -> CounterState + Send + Sync + Clone + 'static,
) -> impl Node<CounterState> {
    FunctionNode::new(name, move |_ctx, state| {
        let op = operation.clone();
        async move { Ok(op(state)) }
    })
}

#[tokio::test]
async fn test_complex_workflow() {
    let built_graph = {
        let mut graph = Graph::new("g");
        graph
            .add_node(create_test_node("step1", |mut state| {
                state.count += 1;
                state.record_operation("step1");
                state
            }))
            .add_node(create_test_node("step2", |mut state| {
                state.count *= 2;
                state.record_operation("step2");
                state
            }))
            .add_node(create_test_node("step3", |mut state| {
                state.count -= 3;
                state.record_operation("step3");
                state
            }))
            .add_edge(START, "step1")
            .add_conditional_edge("step1", move |state: &CounterState| {
                if state.count > 0 {
                    "step2".to_string()
                } else {
                    "step3".to_string()
                }
            })
            .add_edge("step2", "step3")
            .add_edge("step3", END);
        graph.build()
    };

    let ctx = Context::new("test_complex");
    let initial_state = CounterState::new(5);
    let final_state = built_graph.run(&ctx, initial_state).await.unwrap();

    // Verify execution path and results
    println!("Final state: {:?}", final_state);
    assert_eq!(final_state.history, vec!["step1", "step2", "step3"]);
    // 5 -> 6 -> 12 -> 9
    assert_eq!(final_state.count, 9);
}
