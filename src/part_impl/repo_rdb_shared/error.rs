//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};
use serde_json::Error as SerdeJsonError;

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError};

pub(super) fn pool_build(location: &'static str, err: BuildError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] failed to build pool: {}", location, err),
    }
}

pub(super) fn pool_get(location: &'static str, err: PoolError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] failed to get conn: {}", location, err),
    }
}

pub(super) fn diesel(location: &'static str, err: DieselError) -> RootError {
    match err {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => RootError::Expected {
            variant: ExpectedVariant::Conflict,
            message: trl("error-already-exists"),
        },
        DieselError::NotFound => RootError::Unrecoverable {
            message: format!(
                "[{}] unexpected Diesel NotFound; use optional() and map None at call site",
                location,
            ),
        },
        err => RootError::Unrecoverable {
            message: format!("[{}] diesel error: {}", location, err),
        },
    }
}

pub(super) fn serde(location: &'static str, err: SerdeJsonError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] serde error: {}", location, err),
    }
}

pub(super) fn expected(message: &str) -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl(message),
    }
}

pub(super) fn invalid_stored_value(
    location: &'static str,
    value: impl std::fmt::Display,
) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] invalid stored value: {}", location, value),
    }
}
