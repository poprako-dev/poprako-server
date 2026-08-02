//! Application-level error and result types used throughout the domain layer.

use std::error::Error;
use std::result::Result;

use poprako_orchestra::nucl::Error as NuclError;

/// Categorizes an expected application error by its origin domain.
#[derive(Debug)]
pub enum ExpectedVariant {
    //
    /// Invalid or missing arguments.
    Args,

    /// Authentication failure.
    Auth,

    /// perm denied.
    Perm,
}

/// A domain error that is either an expected application condition
/// (invalid arguments, authentication failure, missing perms) or
/// an unrecoverable system-level failure.
#[derive(Debug)]
pub enum BaseError {
    //
    /// An expected application condition — the error can be communicated to the client.
    Expected {
        //
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

impl std::fmt::Display for BaseError {
    // Renders the classified error without exposing additional internal state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expected { message, .. }
            | Self::Unrecoverable { message } => formatter.write_str(message),
        }
    }
}

impl Error for BaseError {}

/// Alias for [`Result`] used at module boundary layers.
pub type BaseRest<T> = Result<T, BaseError>;

/// Wraps a value in `Ok(...)` — the simplest use-case return.
pub fn accept<T>(v: T) -> BaseRest<T> {
    Ok(v)
}

impl<BE, E> From<NuclError<BE, E>> for BaseError
where
    BE: Into<BaseError>,
    E: Into<BaseError>,
{
    // Converts a Nucl error into an application-level Error, unwrapping the backend or step inner error.
    fn from(value: NuclError<BE, E>) -> Self {
        match value {
            //
            NuclError::Backend(error) => error.into(),

            NuclError::Step(error) => error.into(),
        }
    }
}
