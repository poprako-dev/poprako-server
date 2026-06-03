use crate::domain::result::DomainError;
use crate::domain::result::ExpectedVariant;
use crate::usecase::result::UseCaseError;

pub fn is_expected_argument(err: &DomainError) -> bool {
    matches!(
        err,
        DomainError::Expected {
            variant: ExpectedVariant::Argument,
            ..
        }
    )
}

pub fn is_expected_conflict(err: &DomainError) -> bool {
    matches!(
        err,
        DomainError::Expected {
            variant: ExpectedVariant::Conflict,
            ..
        }
    )
}

pub fn usecase_is_expected_argument(err: &UseCaseError) -> bool {
    is_expected_argument(err.as_ref())
}

pub fn usecase_is_expected_conflict(err: &UseCaseError) -> bool {
    is_expected_conflict(err.as_ref())
}

pub fn usecase_is_unrecoverable(err: &UseCaseError) -> bool {
    matches!(err.as_ref(), DomainError::Unrecoverable { .. })
}
