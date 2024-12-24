use super::error::GraphError;

pub type GraphResult<T> = std::result::Result<T, GraphError>;
