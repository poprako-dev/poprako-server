//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};

use poprako_util::i18n::trl;

use crate::result::{BaseError, BaseRest, ExpectedVariant};

/// Converts a persisted signed version into the application's unsigned type.
pub fn version(value: i64) -> BaseRest<u32> {
    u32::try_from(value).map_err(|err| BaseError::Unrecoverable {
        message: format!("invalid persisted version {}: {}", value, err),
    })
}

/// Converts and increments a persisted version without overflowing.
pub fn next_version(value: i64) -> BaseRest<u32> {
    //
    let current_version = version(value)?;

    current_version
        .checked_add(1)
        .ok_or_else(|| BaseError::Unrecoverable {
            message: "persisted version cannot be incremented".into(),
        })
}

/// Converts a pool build error into an unrecoverable `RegularError`.
pub fn pool_build(source: BuildError) -> BaseError {
    BaseError::Unrecoverable {
        message: format!("failed to build pool: {}", source),
    }
}

/// Converts a pool checkout error into an unrecoverable `RegularError`.
pub fn pool_get(source: PoolError) -> BaseError {
    BaseError::Unrecoverable {
        message: format!("failed to get conn: {}", source),
    }
}

/// Converts a Diesel error into the appropriate `RegularError` variant.
///
/// Unique violations and `NotFound` map to `Expected`; all others are `Unrecoverable`.
pub fn diesel(source: DieselError) -> BaseError {
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

        err => BaseError::Unrecoverable {
            message: format!("diesel error: {}", err),
        },
    }
}
