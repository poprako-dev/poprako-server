pub mod hook;
pub mod value_object;

pub mod user;

pub mod result {
    use crate::domain::result::DomainError;
    use crate::util::rename::StdResult;

    #[derive(Debug)]
    pub struct UseCaseError(DomainError);

    impl std::fmt::Display for UseCaseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "UseCaseError({})", self.0)
        }
    }

    impl AsRef<DomainError> for UseCaseError {
        fn as_ref(&self) -> &DomainError {
            &self.0
        }
    }

    impl From<DomainError> for UseCaseError {
        fn from(value: DomainError) -> Self {
            UseCaseError(value)
        }
    }

    pub type UseCaseResult<T> = StdResult<T, UseCaseError>;
}
