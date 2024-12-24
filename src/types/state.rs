use super::error::GraphResult;
use std::any::Any;
use std::fmt::Debug;

pub trait GraphState: Debug + Send + Sync + Any {
    fn merge(&mut self, other: Box<dyn GraphState>) -> GraphResult<()>;
    fn clone_box(&self) -> Box<dyn GraphState>;
}

impl<T> GraphState for T
where
    T: 'static + Debug + Send + Sync + Clone,
{
    fn merge(&mut self, _other: Box<dyn GraphState>) -> GraphResult<()> {
        Ok(()) // Default implementation
    }

    fn clone_box(&self) -> Box<dyn GraphState> {
        Box::new(self.clone())
    }
}
