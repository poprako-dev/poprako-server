//! Application-level error and result types used throughout the domain layer.

use std::result::Result as StdResult;

use poprako_orchestra::nucl::Error as NuclError;

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

/// Convenience alias for [`std::result::Result`] with the application's [`Error`] type.
pub type Result<T> = StdResult<T, Error>;

pub fn accept<T>(v: T) -> Result<T> {
    Ok(v)
}

/// Alias for [`Error`] used at module boundary layers.
pub type RegularError = Error;

/// Alias for [`Result`] used at module boundary layers.
pub type RegularResult<T> = Result<T>;

impl<BE, E> From<NuclError<BE, E>> for Error
where
    BE: Into<Error>,
    E: Into<Error>,
{
    fn from(value: NuclError<BE, E>) -> Self {
        match value {
            NuclError::Backend(error) => error.into(),

            NuclError::Step(error) => error.into(),
        }
    }
}
