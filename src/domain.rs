pub mod actor;
pub mod external;
pub mod hook;
pub mod model;
pub mod query;

pub mod result {
    use crate::util::rename::StdRetVal;

    #[derive(Debug)]
    pub enum ExpectedError {
        /// Validation / resource errors → HTTP 400
        Parameter,
        /// Authentication errors → HTTP 401
        Authentication,
    }

    /// Unified error type for the domain layer.
    ///
    /// - `Expected`: business-logic errors, the message is returned to the end user.
    /// - `Unrecoverable`: internal failures, the message is only used for logging.
    #[derive(Debug)]
    pub enum DomainError {
        Expected {
            variant: ExpectedError,
            message: String,
        },
        Unrecoverable {
            message: String,
        },
    }

    pub type DomainRetVal<T> = StdRetVal<T, DomainError>;
}
