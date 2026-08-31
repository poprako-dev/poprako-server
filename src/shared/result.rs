//! Error conversion helpers for the Diesel-backed repository.

#[cfg(test)]
mod tests;

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use poprako_util::i18n::trl;

use crate::result::{BaseError, ExpectedVariant};

/// Converts a Diesel error into the appropriate `RegularError` variant.
///
/// Unique violations and `NotFound` map to `Expected`; all others are `Unrecoverable`.
pub fn diesel(source: DieselError) -> BaseError {
    //
    match source {
        //
        DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            information,
        ) => {
            //
            let message = trl("error-already-exists");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %message,
                database_err = "unique violation",
                database_message = information.message(),
                database_details = ?information.details(),
                database_hint = ?information.hint(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "expected error constructed from Diesel database error",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            }
        }

        DieselError::DatabaseError(
            DatabaseErrorKind::SerializationFailure,
            information,
        ) => {
            //
            let message = trl("error-concurrent-conflict");

            tracing::warn!(
                database_err = "serialization failure",
                database_message = information.message(),
                database_details = ?information.details(),
                database_hint = ?information.hint(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "retryable error constructed from Diesel database error",
            );

            BaseError::Retryable { message }
        }

        DieselError::NotFound => {
            //
            let message = trl("error-not-found");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %message,
                database_err = "not found",
                "[shared::diesel] unexpected Diesel NotFound; use optional() and map None at call site"
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            }
        }

        err => {
            //
            tracing::error!(
                operation = "execute_database_operation",
                sdk_err = ?err,
                "Diesel SDK error",
            );

            BaseError::Unrecoverable {
                message: format!("diesel error: {}", err),
            }
        }
    }
}
