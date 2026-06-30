//! Application-level error and result types used throughout the domain layer.

use poprako_transactional::drive::result::Error as DriveError;

/// Categorizes an expected application error by its origin domain.
pub enum ExpectedVariant {
    ArgsInvalid,
    AuthFail,
    PermDeny,
    // TODO:
    Conflict,
}

/// A domain error that is either an expected application condition
/// (invalid arguments, authentication failure, missing permissions) or
/// an unrecoverable system-level failure.
pub enum Error {
    Expected {
        variant: ExpectedVariant,
        message: String,
    },
    Unrecoverable {
        message: String,
    },
}

/// Convenience alias for [`std::result::Result`] with the application's [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Wraps a value into the `Ok` variant of [`Result`].
pub fn accept<T>(v: T) -> Result<T> {
    Ok(v)
}

/// Alias for [`Error`] used at module boundary layers.
pub type RootError = Error;

/// Alias for [`Result`] used at module boundary layers.
pub type RootResult<T> = Result<T>;

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
