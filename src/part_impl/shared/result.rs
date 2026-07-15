//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};

use poprako_util::i18n::trl;

use crate::result::{BaseError, BaseResult, ExpectedVariant};

/// Converts a persisted signed version into the application's unsigned type.
pub fn version(value: i64) -> BaseResult<u32> {
    u32::try_from(value).map_err(|err| BaseError::Unrecoverable {
        message: format!("invalid persisted version {}: {}", value, err),
    })
}

/// Converts a pool build error into an unrecoverable `RegularError`.
pub fn pool_build(err: BuildError) -> BaseError {
    BaseError::Unrecoverable {
        message: format!("failed to build pool: {}", err),
    }
}

/// Converts a pool checkout error into an unrecoverable `RegularError`.
pub fn pool_get(err: PoolError) -> BaseError {
    BaseError::Unrecoverable {
        message: format!("failed to get conn: {}", err),
    }
}

/// Converts a Diesel error into the appropriate `RegularError` variant.
///
/// Unique violations and `NotFound` map to `Expected`; all others are `Unrecoverable`.
pub fn diesel(err: DieselError) -> BaseError {
    match err {
        //
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-already-exists"),
            }
        }

        DieselError::NotFound => {
            //
            tracing::warn!(
                "[shared::diesel] unexpected Diesel NotFound; use optional() and map None at call site"
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-not-found"),
            }
        }

        err => BaseError::Unrecoverable {
            message: format!("diesel error: {}", err),
        },
    }
}

/// Creates an `Expected` variant `RegularError` with the given i18n message key.
pub fn expected(message: &str) -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}
