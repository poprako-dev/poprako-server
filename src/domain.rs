pub mod compound;
pub mod effect;
pub mod external;
pub mod model;
pub mod query;

pub mod result {
    use crate::util::rename::StdResult;

    #[derive(Debug)]
    pub enum ExpectedVariant {
        /// Validation / resource errors
        Argument,
        /// Authentication errors
        Authentication,
    }

    /// Unified error type for the domain layer.
    ///
    /// - `Expected`: business-logic errors, the message is returned to the end user.
    /// - `Unrecoverable`: internal failures, the message is only used for logging.
    #[derive(Debug)]
    pub enum DomainError {
        Expected {
            variant: ExpectedVariant,
            message: String,
        },
        Unrecoverable {
            message: String,
        },
    }

    impl DomainError {
        pub fn expected_argument(msg: String) -> Self {
            Self::Expected {
                variant: ExpectedVariant::Argument,
                message: msg,
            }
        }

        pub fn expected_authentication(msg: String) -> Self {
            Self::Expected {
                variant: ExpectedVariant::Authentication,
                message: msg,
            }
        }

        pub fn unrecoverable(msg: String) -> Self {
            Self::Unrecoverable { message: msg }
        }
    }

    impl std::fmt::Display for DomainError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    pub type DomainResult<T> = StdResult<T, DomainError>;
}
