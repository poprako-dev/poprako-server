//! Application-level error and result types used throughout the domain layer.

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
        msg: String,
    },

    /// A transient concurrency conflict that the caller may retry.
    Retryable {
        /// Human-readable detail describing the retryable condition.
        msg: String,
    },

    /// A temporarily unavailable dependency that the caller may try again.
    Unavailable {
        /// Generic availability detail retained for non-HTTP callers.
        msg: String,
    },

    /// An unexpected system-level failure — cannot be recovered mid-request.
    Unrecoverable {
        /// Description of the system failure.
        msg: String,
    },
}

impl BaseError {
    /// Constructs and records an expected error at its unlogged source.
    /// Already-logged errors must retain their variant when propagated.
    #[must_use]
    #[track_caller]
    pub fn expected(variant: ExpectedVariant, msg: String) -> Self {
        //
        let origin = std::panic::Location::caller();

        tracing::warn!(
            err_variant = ?variant,
            err_msg = %msg,
            source_file = origin.file(),
            source_line = origin.line(),
            "expected application error",
        );

        Self::Expected { variant, msg }
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
                msg: "database connection capacity is temporarily unavailable"
                    .into(),
            },

            source => Self::Unrecoverable {
                msg: source.to_string(),
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
            ObjDeptError::Invalid { msg } => Self::Expected {
                variant: ExpectedVariant::Args,
                msg,
            },

            ObjDeptError::Conflict { msg }
            | ObjDeptError::Retryable { msg } => Self::Retryable { msg },

            ObjDeptError::Unavailable { msg } => Self::Unavailable { msg },

            ObjDeptError::Unrecoverable { msg } => Self::Unrecoverable { msg },
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
                msg: trl("error-already-exists"),
            },

            PromError::ConcurrentConflict => Self::Retryable {
                msg: trl("error-concurrent-conflict"),
            },

            PromError::NotFound => Self::Expected {
                variant: ExpectedVariant::Args,
                msg: trl("error-not-found"),
            },

            PromError::Unrecoverable { msg } => Self::Unrecoverable { msg },
        }
    }
}
