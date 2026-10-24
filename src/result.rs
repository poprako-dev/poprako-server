//! Application-level error and result types used throughout the domain layer.

// Error constructors emit diagnostics at the original failure boundary.
#[macro_use]
mod macros;

#[cfg(test)]
mod tests;

use std::result::Result;

use poprako_obj_dept::rest::ObjDeptError;
use poprako_prom::general::rdb_impl::PromError;
use poprako_rdb_core::RdbError;
use poprako_util::i18n::trl;

/// Categorizes an expected application error by its origin domain.
#[derive(Clone, Copy, Debug)]
pub enum ExpectedVariant {
    /// Invalid or missing arguments.
    Args,

    /// Authentication failure.
    Auth,

    /// Perm denied.
    Perm,
}

/// Classifies expected client conditions, retryable conflicts, unavailable
/// dependencies, and unrecoverable internal failures.
#[derive(Debug)]
pub enum BaseError {
    /// An expected application condition — the error can be communicated to the client.
    Expected {
        /// Classification of the expected error variant.
        variant: ExpectedVariant,
        /// Human-readable detail about the error condition.
        message: String,
    },

    /// A transient concurrency conflict that the caller may retry.
    Retryable {
        /// Safe client-visible detail describing the retryable condition.
        message: String,
    },

    /// A temporarily unavailable dependency that the caller may try again.
    Unavailable {
        /// Generic availability detail retained for non-HTTP callers.
        message: String,
    },

    /// An unexpected system-level failure — cannot be recovered mid-request.
    Unrecoverable {
        /// Safe diagnostic for logging; HTTP responses conceal this detail.
        message: String,
    },
}

/// Alias for [`Result`] used at module boundary layers.
pub type BaseRest<T> = Result<T, BaseError>;

impl From<RdbError> for BaseError {
    // Convert a traced RDB infrastructure failure into the application error surface.
    fn from(source: RdbError) -> Self {
        //
        match source {
            //
            RdbError::PoolWaitTimeout => Self::Unavailable {
                message:
                    "database connection capacity is temporarily unavailable"
                        .into(),
            },

            source => Self::Unrecoverable {
                message: source.to_string(),
            },
        }
    }
}

impl From<ObjDeptError> for BaseError {
    // Preserve the object-department error classification at the application boundary.
    fn from(source: ObjDeptError) -> Self {
        //
        match source {
            //
            ObjDeptError::Invalid { message } => Self::Expected {
                variant: ExpectedVariant::Args,
                message,
            },

            ObjDeptError::Conflict { message }
            | ObjDeptError::Retryable { message } => {
                Self::Retryable { message }
            }

            ObjDeptError::Unavailable { message } => {
                Self::Unavailable { message }
            }

            ObjDeptError::Unrecoverable { message } => {
                Self::Unrecoverable { message }
            }
        }
    }
}

impl From<PromError> for BaseError {
    // Preserve the classification recorded at the neutral Prom adapter leaf.
    fn from(source: PromError) -> Self {
        //
        match source {
            //
            PromError::AlreadyExists => Self::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-already-exists"),
            },

            PromError::ConcurrentConflict => Self::Retryable {
                message: trl("error-concurrent-conflict"),
            },

            PromError::NotFound => Self::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-not-found"),
            },

            PromError::Unrecoverable { message } => {
                Self::Unrecoverable { message }
            }
        }
    }
}
