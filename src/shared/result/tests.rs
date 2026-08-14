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
