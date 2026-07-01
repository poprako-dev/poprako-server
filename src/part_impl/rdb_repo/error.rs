//! Error conversion helpers for the Diesel-backed repository.

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::deadpool::{BuildError, PoolError};
use serde_json::Error as SerdeJsonError;

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError};

pub fn pool_build(err: BuildError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[RdbRepo::from_database_url] failed to build pool: {}", err),
    }
}

pub fn pool_get(location: &'static str, err: PoolError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] failed to get connection: {}", location, err),
    }
}

pub fn diesel(err: DieselError) -> RootError {
    match err {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => RootError::Expected {
            variant: ExpectedVariant::Conflict,
            message: trl("error-already-exists"),
        },
        DieselError::NotFound => RootError::Unrecoverable {
            message:
                "[RdbRepo] unexpected Diesel NotFound; use optional() and map None at call site"
                    .into(),
        },
        err => RootError::Unrecoverable {
            message: format!("[RdbRepo] diesel error: {}", err),
        },
    }
}

pub fn serde(location: &'static str, err: SerdeJsonError) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] serde error: {}", location, err),
    }
}

pub fn expected(message: &str) -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl(message),
    }
}

pub fn invalid_stored_value(location: &'static str, value: impl std::fmt::Display) -> RootError {
    RootError::Unrecoverable {
        message: format!("[{}] invalid stored value: {}", location, value),
    }
}
