use crate::domain::query::QueryError;
use crate::util::rename::StdRetVal;

pub mod user;

#[derive(Debug)]
pub enum ActorError {
    NotFound,
    Conflict,
    Unexpected(String),
}

impl From<QueryError> for ActorError {
    fn from(e: QueryError) -> Self {
        match e {
            QueryError::NotFound => Self::NotFound,
            QueryError::Conflict => Self::Conflict,
            QueryError::Unrecoverable(m) => Self::Unexpected(m),
        }
    }
}

pub type ActorRetVal<T> = StdRetVal<T, ActorError>;
