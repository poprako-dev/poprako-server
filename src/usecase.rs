pub mod hook;
pub mod value_object;

pub mod user;

pub mod result {
    use crate::domain::result::DomainError;
    use crate::util::rename::StdRetVal;

    pub enum UseCaseError {
        Params(String),
        Unrecoverable(String),
    }

    impl From<DomainError> for UseCaseError {
        fn from(value: DomainError) -> Self {
            match value {
                DomainError::Expected { message, .. } => UseCaseError::Params(message),
                DomainError::Unrecoverable { message } => UseCaseError::Unrecoverable(message),
            }
        }
    }

    pub type UseCaseRetVal<T> = StdRetVal<T, UseCaseError>;
}
