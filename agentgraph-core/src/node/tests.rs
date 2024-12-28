#[cfg(test)]
mod tests {
    use agentgraph_core::*;
    use agentgraph_macros::State;

    #[derive(State, Debug, Clone, PartialEq)]
    struct TestState {
        #[update(replace)]
        name: String,
    }

    #[tokio::test]
    async fn test_function_node() {
        let node = FunctionNode::new("test", |_ctx, _: TestState| async move {
            Ok(NodeOutput::Full(TestState {
                name: "Ryan".to_string(),
            }))
        });

        let ctx = Context::new("test");
        let result = node
            .process(
                &ctx,
                TestState {
                    name: "test".to_string(),
                },
            )
            .await
            .unwrap();

        match result {
            NodeOutput::Full(state) => {
                assert_eq!(
                    state,
                    TestState {
                        name: "Ryan".to_string()
                    }
                );
            }
            NodeOutput::Updates(updates) => {
                panic!("Expected a full state, but got updates: {:?}", updates);
            }
        }

        assert_eq!(node.name(), "test");
    }
}
