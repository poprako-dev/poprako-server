//! Error conversion helpers for the Diesel-backed repository.

#[cfg(test)]
mod tests;

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use poprako_util::i18n::trl;

use crate::result::{BaseError, ExpectedVariant};

/// Converts a Diesel error into the appropriate `BaseError` variant.
///
/// Unique violations and `NotFound` map to `Expected`, serialization failures
/// are retryable, and unclassified database failures remain unrecoverable.
pub fn diesel(source: DieselError) -> BaseError {
    //
    match source {
        //
        DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            information,
        ) => {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-already-exists");
                database_error = "unique violation",
                database_message = information.message(),
                database_details = ?information.details(),
                database_hint = ?information.hint(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "database uniqueness constraint violated",
            }
        }

        DieselError::DatabaseError(
            DatabaseErrorKind::SerializationFailure,
            information,
        ) => {
            //
            retryable_error! {
                message: trl("error-concurrent-conflict");
                database_error = "serialization failure",
                database_message = information.message(),
                database_details = ?information.details(),
                database_hint = ?information.hint(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "database transaction conflict",
            }
        }

        DieselError::NotFound => {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-not-found");
                database_error = "not found",
                "[shared::diesel] unexpected Diesel NotFound; use optional() \
                    and map None at call site",
            }
        }

        error => {
            //
            unrecoverable_error! {
                message: format!("diesel error: {}", error);
                operation = "execute_database_operation",
                sdk_error = ?error,
                "Diesel SDK error",
            }
        }
    }
}
