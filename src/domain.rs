pub mod actor;
pub mod external;
pub mod hook;
pub mod model;
pub mod query;

pub mod result {
    use crate::domain::actor::ActorError;
    use crate::domain::query::QueryError;
    use crate::util::rename::StdRetVal;

    // `RunError` represents any error that may be encountered
    // in a `run_with` function.
    #[derive(Debug)]
    pub enum DomainError {
        Expected(String),
        Unrecoverable(String),
    }

    impl From<QueryError> for DomainError {
        fn from(e: QueryError) -> Self {
            match e {
                QueryError::NotFound => Self::Expected("not found".to_string()),
                QueryError::Conflict => Self::Expected("conflict".to_string()),
                QueryError::Unrecoverable(m) => Self::Unrecoverable(m),
            }
        }
    }

    impl From<ActorError> for DomainError {
        fn from(e: ActorError) -> Self {
            match e {
                ActorError::Expected(m) => Self::Expected(m),
                ActorError::Unrecoverable(m) => Self::Unrecoverable(m),
            }
        }
    }

    pub type DomainRetVal<T> = StdRetVal<T, DomainError>;
}
