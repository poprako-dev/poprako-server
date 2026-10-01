//! Application-level error and result types used throughout the domain layer.

use std::result::Result;

use poprako_obj_dept::rest::ObjDeptError;
use poprako_rdb_core::RdbError;

/// Categorizes an expected application error by its origin domain.
#[derive(Clone, Copy, Debug)]
pub enum ExpectedVariant {
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
    /// An expected application condition — the error can be communicated to the client.
    Expected {
        /// Classification of the expected error variant.
        variant: ExpectedVariant,
        /// Human-readable detail about the error condition.
        message: String,
    },

    /// A transient concurrency conflict that the caller may retry.
    Retryable {
        /// Human-readable detail describing the retryable condition.
        message: String,
    },

    /// A temporarily unavailable dependency that the caller may try again.
    Unavailable {
        /// Generic availability detail retained for non-HTTP callers.
        message: String,
    },

    /// An unexpected system-level failure — cannot be recovered mid-request.
    Unrecoverable {
        /// Description of the system failure.
        message: String,
    },
}

impl BaseError {
    /// Constructs and records an expected error at its unlogged source.
    /// Already-logged errors must retain their variant when propagated.
    #[must_use]
    #[track_caller]
    pub fn expected(variant: ExpectedVariant, message: String) -> Self {
        //
        let origin = std::panic::Location::caller();

        tracing::warn!(
            err_variant = ?variant,
            err_message = %message,
            source_file = origin.file(),
            source_line = origin.line(),
            "expected application error",
        );

        Self::Expected { variant, message }
    }
}

/// Alias for [`Result`] used at module boundary layers.
pub type BaseRest<T> = Result<T, BaseError>;

/// Wraps a value in `Ok(...)` — the simplest use-case return.
pub const fn accept<T>(v: T) -> BaseRest<T> {
    Ok(v)
}

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
