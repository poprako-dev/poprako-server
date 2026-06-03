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
        /// Resource conflict errors (e.g. unique constraint violations)
        Conflict,
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

        pub fn expected_conflict(msg: String) -> Self {
            Self::Expected {
                variant: ExpectedVariant::Conflict,
                message: msg,
            }
        }

        pub fn unrecoverable(msg: String) -> Self {
            Self::Unrecoverable { message: msg }
        }

        pub fn trace(self) -> Self {
            match &self {
                Self::Expected { variant, message } => {
                    tracing::debug!(
                        variant = ?variant,
                        message = %message,
                        "[DomainError] expected error produced",
                    );
                }
                Self::Unrecoverable { message } => {
                    tracing::error!(
                        error = %message,
                        "[DomainError] unrecoverable error produced",
                    );
                }
            }

            self
        }
    }

    pub type DomainResult<T> = StdResult<T, DomainError>;

    #[cfg(test)]
    mod tests {
        // expected_argument_variant(DomainError::expected_argument)(positive): expected argument errors should carry the Argument variant and message.
        // expected_authentication_variant(DomainError::expected_authentication)(positive): expected authentication errors should carry the Authentication variant and message.
        // expected_conflict_variant(DomainError::expected_conflict)(positive): expected conflict errors should carry the Conflict variant and message.
        // unrecoverable_variant(DomainError::unrecoverable)(positive): unrecoverable errors should carry their diagnostic message.
        use super::DomainError;
        use super::ExpectedVariant;

        #[test]
        fn expected_argument_variant() {
            let err = DomainError::expected_argument("bad input".into());
            match err {
                DomainError::Expected { variant, message } => {
                    assert!(matches!(variant, ExpectedVariant::Argument));
                    assert_eq!(message, "bad input");
                }
                _ => panic!("expected Expected variant"),
            }
        }

        #[test]
        fn expected_authentication_variant() {
            let err = DomainError::expected_authentication("no access".into());
            match err {
                DomainError::Expected { variant, message } => {
                    assert!(matches!(variant, ExpectedVariant::Authentication));
                    assert_eq!(message, "no access");
                }
                _ => panic!("expected Expected variant"),
            }
        }

        #[test]
        fn expected_conflict_variant() {
            let err = DomainError::expected_conflict("duplicate".into());
            match err {
                DomainError::Expected { variant, message } => {
                    assert!(matches!(variant, ExpectedVariant::Conflict));
                    assert_eq!(message, "duplicate");
                }
                _ => panic!("expected Expected variant"),
            }
        }

        #[test]
        fn unrecoverable_variant() {
            let err = DomainError::unrecoverable("boom".into());
            match err {
                DomainError::Unrecoverable { message } => {
                    assert_eq!(message, "boom");
                }
                _ => panic!("expected Unrecoverable variant"),
            }
        }
    }
}
