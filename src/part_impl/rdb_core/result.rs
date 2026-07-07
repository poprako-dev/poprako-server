//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RegularError};

pub fn pool_build(err: BuildError) -> RegularError {
    RegularError::Unrecoverable {
        message: format!("failed to build pool: {}", err),
    }
}

pub fn pool_get(err: PoolError) -> RegularError {
    RegularError::Unrecoverable {
        message: format!("failed to get conn: {}", err),
    }
}

pub fn diesel(err: DieselError) -> RegularError {
    match err {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            RegularError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-already-exists"),
            }
        }
        DieselError::NotFound => {
            tracing::warn!(
                "[rdb_core::diesel] unexpected Diesel NotFound; use optional() and map None at call site"
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

pub fn expected(message: &str) -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}
