//! Application-level error and result types used throughout the domain layer.

use std::result::Result as StdResult;

use poprako_transactional::drive::result::Error as DriveError;

/// Categorizes an expected application error by its origin domain.
#[derive(Debug)]
pub enum ExpectedVariant {
    Args,
    Auth,
    Perm,
}

/// A domain error that is either an expected application condition
/// (invalid arguments, authentication failure, missing permissions) or
/// an unrecoverable system-level failure.
#[derive(Debug)]
pub enum Error {
    Expected {
        variant: ExpectedVariant,
        message: String,
    },
    Unrecoverable {
        message: String,
    },
}

impl Error {
    /// Constructs an `Error::Expected` with an `Args` variant.
    pub fn expected_args(_msg: String) -> Self {
        todo!()
    }

    /// Constructs an `Error::Expected` with an `Auth` variant.
    pub fn expected_auth(_msg: String) -> Self {
        todo!()
    }

    /// Constructs an `Error::Expected` with a `Perm` variant.
    pub fn expected_perm(_msg: String) -> Self {
        todo!()
    }

    /// Constructs an `Error::Unrecoverable` with the given message.
    pub fn unrecoverable(_msg: String) -> Self {
        todo!()
    }
}

/// Convenience alias for [`std::result::Result`] with the application's [`Error`] type.
pub type Result<T> = StdResult<T, Error>;

/// Alias for [`Error`] used at module boundary layers.
pub type RegularError = Error;

/// Alias for [`Result`] used at module boundary layers.
pub type RegularResult<T> = Result<T>;

impl<E, BE> From<DriveError<E, BE>> for Error
where
    E: Into<Error>,
    BE: Into<Error>,
{
    fn from(value: DriveError<E, BE>) -> Self {
        match value {
            DriveError::Advance(e) => e.into(),
            DriveError::Backend(e) => e.into(),
        }
    }
}
