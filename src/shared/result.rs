//! Error conversion helpers for the Diesel-backed repository.

#[cfg(test)]
mod tests;

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use poprako_util::i18n::trl;

use crate::result::{BaseError, ExpectedVariant};

/// Converts a Diesel error into the appropriate `RegularError` variant.
///
/// Unique violations and `NotFound` map to `Expected`; all others are `Unrecoverable`.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
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
                database_err = "unique violation",
                database_msg = information.message(),
                database_details = ?information.details(),
                database_hint = ?information.hint(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "expected error constructed from Diesel database error",
            }
        }

        DieselError::DatabaseError(
            DatabaseErrorKind::SerializationFailure,
            information,
        ) => {
            //
            let msg = trl("error-concurrent-conflict");

            tracing::warn!(
                database_err = "serialization failure",
                database_msg = information.message(),
                database_details = ?information.details(),
                database_hint = ?information.hint(),
                constraint = ?information.constraint_name(),
                table = ?information.table_name(),
                column = ?information.column_name(),
                "retryable error constructed from Diesel database error",
            );

            BaseError::Retryable { msg }
        }

        DieselError::NotFound => {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-not-found");
                database_err = "not found",
                "[shared::diesel] unexpected Diesel NotFound; use optional() \
                    and map None at call site",
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
                msg: format!("diesel error: {}", err),
            }
        }
    }
}
