use super::result::GraphResult;
use std::any::Any;
use std::fmt::Debug;

pub trait UpdateableState {
    type Update;

    /// Update this state with a single update.
    fn update(&mut self, update: Self::Update);

    /// Optionally: apply multiple updates in sequence.
    fn update_many<I: IntoIterator<Item = Self::Update>>(&mut self, updates: I) {
        for update in updates {
            self.update(update);
        }
    }
}

pub trait GraphState: Debug + Send + Sync + Any {
    fn merge(&mut self, other: Box<dyn GraphState>) -> GraphResult<()>;
    fn clone_box(&self) -> Box<dyn GraphState>;
}

impl<T> GraphState for T
where
    T: 'static + Debug + Send + Sync + Clone + UpdateableState,
{
    fn merge(&mut self, _other: Box<dyn GraphState>) -> GraphResult<()> {
        Ok(()) // Default implementation
    }

    fn clone_box(&self) -> Box<dyn GraphState> {
        Box::new(self.clone())
    }
}
