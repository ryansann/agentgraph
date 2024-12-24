#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_function_node() {
        let node = FunctionNode::new("test", |_ctx, state: i32| async move { Ok(state + 1) });

        let ctx = Context::new("test");
        let result = node.process(&ctx, 1).await.unwrap();

        assert_eq!(result, 2);
        assert_eq!(node.name(), "test");
    }
}
