//! Application-level error and result types used throughout the domain layer.

use std::result::Result as StdResult;

use poprako_orchestra::nucl::Error as NuclError;

/// Categorizes an expected application error by its origin domain.
#[derive(Debug)]
pub enum ExpectedVariant {
    /// Invalid or missing arguments.
    Args,
    /// Authentication failure.
    Auth,
    /// Permission denied.
    Perm,
}

/// A domain error that is either an expected application condition
/// (invalid arguments, authentication failure, missing permissions) or
/// an unrecoverable system-level failure.
#[derive(Debug)]
pub enum Error {
    /// An expected application condition — the error can be communicated to the client.
    Expected {
        /// Classification of the expected error variant.
        variant: ExpectedVariant,
        /// Human-readable detail about the error condition.
        message: String,
    },
    /// An unexpected system-level failure — cannot be recovered mid-request.
    Unrecoverable {
        /// Description of the system failure.
        message: String,
    },
}

/// Convenience alias for [`std::result::Result`] with the application's [`Error`] type.
pub type Result<T> = StdResult<T, Error>;

/// Wraps a value in `Ok(...)` — the simplest use-case return.
pub fn accept<T>(v: T) -> Result<T> {
    Ok(v)
}

/// Alias for [`Error`] used at module boundary layers.
pub type BaseError = Error;

/// Alias for [`Result`] used at module boundary layers.
pub type BaseResult<T> = Result<T>;

impl<BE, E> From<NuclError<BE, E>> for Error
where
    BE: Into<Error>,
    E: Into<Error>,
{
    fn from(value: NuclError<BE, E>) -> Self {
        match value {
            //
            NuclError::Backend(error) => error.into(),

            NuclError::Step(error) => error.into(),
        }
    }
}
