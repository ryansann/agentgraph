#[cfg(test)]
mod tests {
    use crate::*;
    use agentgraph_macros::State;

    #[derive(State, Debug, Clone)]
    struct CounterState {
        #[update(replace)]
        count: i32,

        #[update(append)]
        operations: Vec<String>,
    }

    impl Default for CounterState {
        fn default() -> Self {
            CounterState {
                count: 0,
                operations: vec![],
            }
        }
    }

    #[test]
    fn test_counter_state_replace() {
        // The macro expanded to an impl of UpdateableState for CounterState
        let mut state = CounterState::default();
        state.apply(CounterStateUpdate::Count(5));
        assert_eq!(state.count, 5);
    }

    #[test]
    fn test_counter_state_append() {
        let mut state = CounterState::default();
        state.apply(CounterStateUpdate::Operations(
            vec!["increment".to_string()],
        ));
        state.apply(CounterStateUpdate::Operations(
            vec!["decrement".to_string()],
        ));
        assert_eq!(state.operations, vec!["increment", "decrement"]);
    }
}
