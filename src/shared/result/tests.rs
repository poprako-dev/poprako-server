use super::diesel;

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::result::BaseError;

#[test]
fn serialization_failure_is_retryable() {
    let source = DieselError::DatabaseError(
        DatabaseErrorKind::SerializationFailure,
        Box::new("could not serialize access".to_string()),
    );

    let error = diesel(source);

    assert!(matches!(error, BaseError::Retryable { .. }));
}

#[test]
// unknown_database_error_is_not_assumed_retryable(diesel)(negative): an unknown driver error must not become a retryable conflict.
fn unknown_database_error_is_not_assumed_retryable() {
    let source = DieselError::DatabaseError(
        DatabaseErrorKind::Unknown,
        Box::new("unclassified database failure".to_string()),
    );

    let error = diesel(source);

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}
