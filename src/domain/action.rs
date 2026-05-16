use crate::{domain::query, util::rename::StdResult};

pub mod user;

#[derive(Debug)]
pub enum Error {
    NotFound,
    Conflict(String),
    Unexpected(String),
}

impl From<query::QueryError> for Error {
    fn from(e: query::QueryError) -> Self {
        match e {
            query::QueryError::NotFound => Self::NotFound,
            query::QueryError::Conflict => Self::Conflict("Conflict".to_string()),
            query::QueryError::Unrecoverable(m) => Self::Unexpected(m),
        }
    }
}

pub type Result<T> = StdResult<T, Error>;
