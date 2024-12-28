#[cfg(test)]
mod tests {
    use agentgraph_core::*;
    use agentgraph_macros::State;
    use std::sync::Arc;

    #[derive(State, Debug, Clone, PartialEq)]
    struct CounterState {
        #[update(replace)]
        count: i32,
    }

    #[tokio::test]
    async fn test_basic_graph() {
        let node1 = FunctionNode::new("node1", |_ctx, state: CounterState| async move {
            Ok(NodeOutput::Updates(vec![CounterStateUpdate::Count(
                state.count + 1,
            )]))
        });

        let node2 = FunctionNode::new("node2", |_ctx, state: CounterState| async move {
            Ok(NodeOutput::Full(CounterState {
                count: state.count * 2,
            }))
        });
        // Build graph
        let built_graph = {
            let mut graph = Graph::new("g");
            graph
                .add_node(node1)
                .add_node(node2)
                .add_edge("node1", "node2")
                .add_edge(START, "node1")
                .add_edge("node2", END);
            graph.build()
        };

        // Run graph
        let ctx = Context::new("test");
        let result = built_graph
            .run(&ctx, CounterState { count: 1 })
            .await
            .unwrap();

        // 1 + 1 = 2, 2 * 2 = 4
        assert_eq!(result.count, 4);
    }

    #[tokio::test]
    async fn test_conditional_graph() {
        let node1 = FunctionNode::new("node1", |_ctx, state: CounterState| async move {
            Ok(NodeOutput::Updates(vec![CounterStateUpdate::Count(
                state.count + 1,
            )]))
        });

        let node2 = FunctionNode::new("node2", |_ctx, state: CounterState| async move {
            Ok(NodeOutput::Full(CounterState {
                count: state.count * 2,
            }))
        });

        // Build graph with condition
        let built_graph = {
            let mut graph = Graph::new("g");
            graph
                .add_node(create_add_1_node())
                .add_node(create_multiply_2_node())
                .add_edge(START, "node1")
                .add_edge("node2", END)
                .add_conditional_edge("node1", |state: &CounterState| {
                    if state.count < 5 {
                        "node2".into()
                    } else {
                        END.into()
                    }
                });
            graph.build()
        };

        // Test when condition routes to node2
        let ctx = Context::new("test1");
        let result = built_graph
            .run(&ctx, CounterState { count: 1 })
            .await
            .unwrap();
        assert_eq!(result.count, 4);

        // Test when condition routes to END
        let ctx = Context::new("test2");
        let result = built_graph
            .run(&ctx, CounterState { count: 5 })
            .await
            .unwrap();
        assert_eq!(result.count, 6);
    }

    // Test edge creation and debug formatting
    #[test]
    fn test_edge_creation_and_debug() {
        // Test Direct edge
        let direct_edge: Edge<CounterState> = Edge::Direct("next".to_string());
        assert!(format!("{:?}", direct_edge).contains("Direct"));

        // Test Conditional edge
        let condition: Condition<CounterState> = Arc::new(|state: &CounterState| {
            if state.count > 5 {
                "high".to_string()
            } else {
                "low".to_string()
            }
        });
        let cond_edge: Edge<CounterState> = Edge::Conditional(condition);
        assert!(format!("{:?}", cond_edge).contains("Conditional"));
    }

    // Test conditional edge execution
    #[test]
    fn test_conditional_edge() {
        let condition: Condition<CounterState> = Arc::new(|state: &CounterState| {
            if state.count > 5 {
                "high".to_string()
            } else {
                "low".to_string()
            }
        });

        let state = CounterState { count: 10 };
        assert_eq!(condition(&state), "high");

        let state = CounterState { count: 3 };
        assert_eq!(condition(&state), "low");
    }

    // Test edge cloning
    #[test]
    fn test_edge_cloning() {
        let direct_edge: Edge<CounterState> = Edge::Direct("next".to_string());
        let cloned_direct = direct_edge.clone();

        match cloned_direct {
            Edge::Direct(target) => assert_eq!(target, "next"),
            _ => panic!("Wrong edge type after cloning"),
        }

        let condition: Condition<CounterState> = Arc::new(|_| "test".to_string());
        let cond_edge: Edge<CounterState> = Edge::Conditional(condition);
        let cloned_cond = cond_edge.clone();

        match cloned_cond {
            Edge::Conditional(c) => assert_eq!(c(&CounterState { count: 0 }), "test"),
            _ => panic!("Wrong edge type after cloning"),
        }
    }
}
