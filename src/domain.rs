pub mod actor;
pub mod external;
pub mod hook;
pub mod model;
pub mod query;

pub mod result {
    use crate::util::rename::StdResl;

    #[derive(Debug)]
    pub enum ExpectedErr {
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
    pub enum DomainErr {
        Expected {
            variant: ExpectedErr,
            message: String,
        },
        Unrecoverable {
            message: String,
        },
    }

    impl DomainErr {
        pub fn expected_argument(msg: String) -> Self {
            Self::Expected {
                variant: ExpectedErr::Argument,
                message: msg,
            }
        }

        pub fn expected_authentication(msg: String) -> Self {
            Self::Expected {
                variant: ExpectedErr::Authentication,
                message: msg,
            }
        }

        pub fn unrecoverable(msg: String) -> Self {
            Self::Unrecoverable { message: msg }
        }
    }

    impl std::fmt::Display for DomainErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    pub type DomainResl<T> = StdResl<T, DomainErr>;
}
