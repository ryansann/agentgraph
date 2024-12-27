#[cfg(test)]
mod tests {
    use crate::*;
    use agentgraph_macros::State;

    #[derive(State, Debug, Clone)]
    struct CounterState {
        count: i32,
    }

    fn test_graph_state() {
        let mut state = CounterState { count: 0 };
        let other_state = CounterState { count: 5 };

        // Test cloning through GraphState trait
        let boxed_state: Box<dyn GraphState> = Box::new(state.clone());

        // Test merging
        state.merge(Box::new(other_state)).unwrap();
    }
}
