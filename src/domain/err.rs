use crate::domain::query::QueryError;
use crate::util::rename::StdRetVal;

// `RunError` represents any error that may be encountered
// in a `run_with` function.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    // Error from the query layer, such as connection errors or not found errors.
    #[error("query error: {0:?}")]
    Query(#[from] QueryError),
    // No auto wrapping, as it leads to a conflicting impl.
    // Business error from the domain layer, such as validation errors or other domain-specific errors.
    #[error("business error: {0:?}")]
    Business(String),
}

pub type DomainRetVal<T> = StdRetVal<T, DomainError>;
