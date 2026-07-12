//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RegularError, RegularResult};

/// Converts a persisted signed version into the application's unsigned type.
pub fn version(value: i64) -> RegularResult<u32> {
    u32::try_from(value).map_err(|err| RegularError::Unrecoverable {
        message: format!("invalid persisted version {}: {}", value, err),
    })
}

/// Converts a pool build error into an unrecoverable `RegularError`.
pub fn pool_build(err: BuildError) -> RegularError {
    RegularError::Unrecoverable {
        message: format!("failed to build pool: {}", err),
    }
}

/// Converts a pool checkout error into an unrecoverable `RegularError`.
pub fn pool_get(err: PoolError) -> RegularError {
    RegularError::Unrecoverable {
        message: format!("failed to get conn: {}", err),
    }
}

/// Converts a Diesel error into the appropriate `RegularError` variant.
///
/// Unique violations and `NotFound` map to `Expected`; all others are `Unrecoverable`.
pub fn diesel(err: DieselError) -> RegularError {
    match err {
        //
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            RegularError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-already-exists"),
            }
        }

        DieselError::NotFound => {
            //
            tracing::warn!(
                "[shared::diesel] unexpected Diesel NotFound; use optional() and map None at call site"
            );

            RegularError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-not-found"),
            }
        }

        err => RegularError::Unrecoverable {
            message: format!("diesel error: {}", err),
        },
    }
}

/// Creates an `Expected` variant `RegularError` with the given i18n message key.
pub fn expected(message: &str) -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}
