use crate::{GraphError, GraphState, NodeError};
use std::cmp::PartialEq;
use std::result::Result;

#[derive(PartialEq, Debug)]
pub enum NodeOutput<S>
where
    S: GraphState,
{
    /// The node has produced an entirely new state.
    Full(S),

    /// The node has produced zero or more updates to the existing state.
    Updates(Vec<S::Update>),
}

pub type NodeResult<S> = Result<NodeOutput<S>, NodeError>;

pub type GraphResult<T> = Result<T, GraphError>;
