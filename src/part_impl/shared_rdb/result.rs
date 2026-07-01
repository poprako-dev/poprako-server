//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};
use serde_json::Error as SerdeJsonError;

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError};

pub fn pool_build(err: BuildError) -> RootError {
    RootError::Unrecoverable {
        message: format!("failed to build pool: {}", err),
    }
}

pub fn pool_get(err: PoolError) -> RootError {
    RootError::Unrecoverable {
        message: format!("failed to get conn: {}", err),
    }
}

pub fn diesel(err: DieselError) -> RootError {
    match err {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => RootError::Expected {
            variant: ExpectedVariant::Conflict,
            message: trl("error-already-exists"),
        },
        DieselError::NotFound => RootError::Unrecoverable {
            message: format!(
                "unexpected Diesel NotFound; use optional() and map None at call site",
            ),
        },
        err => RootError::Unrecoverable {
            message: format!("diesel error: {}", err),
        },
    }
}

pub fn serde(err: SerdeJsonError) -> RootError {
    RootError::Unrecoverable {
        message: format!("serde error: {}", err),
    }
}

pub fn expected(message: &str) -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl(message),
    }
}

pub fn invalid_stored_value(value: impl std::fmt::Display) -> RootError {
    RootError::Unrecoverable {
        message: format!("invalid stored value: {}", value),
    }
}
