pub mod hook;
pub mod val;

pub mod user;

pub mod result {
    use crate::domain::actor::ActorError;
    use crate::domain::result::DomainError;
    use crate::util::rename::StdRetVal;

    pub enum UseCaseError {
        Params(String),
        Unrecoverable(String),
    }

    impl From<DomainError> for UseCaseError {
        fn from(value: DomainError) -> Self {
            match value {
                DomainError::Expected(m) => UseCaseError::Params(m),
                DomainError::Unrecoverable(m) => UseCaseError::Unrecoverable(m),
            }
        }
    }

    impl From<ActorError> for UseCaseError {
        fn from(value: ActorError) -> Self {
            match value {
                ActorError::Expected(m) => UseCaseError::Params(m),
                ActorError::Unrecoverable(m) => UseCaseError::Unrecoverable(m),
            }
        }
    }

    pub type UseCaseRetVal<T> = StdRetVal<T, UseCaseError>;
}
