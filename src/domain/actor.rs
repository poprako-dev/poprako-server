use crate::domain::query::QueryError;
use crate::util::rename::StdRetVal;

pub mod user;

#[derive(Debug)]
pub enum ActorError {
    Expected(String),
    Unrecoverable(String),
}

impl From<QueryError> for ActorError {
    fn from(e: QueryError) -> Self {
        match e {
            QueryError::NotFound => Self::Expected("未找到对应记录".to_string()),
            QueryError::Conflict => Self::Expected("与已有记录冲突".to_string()),
            QueryError::Unrecoverable(m) => Self::Unrecoverable(m),
        }
    }
}

pub type ActorRetVal<T> = StdRetVal<T, ActorError>;
