pub mod data_object;

pub mod member;
pub mod team;
pub mod user;
pub mod workset;

pub mod result {
    use poprako_util::rename::StdResult;

    use crate::domain::result::DomainError;

    #[derive(Debug)]
    pub struct UseCaseError(DomainError);

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

    impl std::fmt::Display for UseCaseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    pub type UseCaseResult<T> = StdResult<T, UseCaseError>;
}
