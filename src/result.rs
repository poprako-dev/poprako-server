//! Application-level error and result types used throughout the domain layer.

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

/// Alias for [`Result`] used at module boundary layers.
pub type BaseRest<T> = std::result::Result<T, BaseError>;

/// Wraps a value in `Ok(...)` — the simplest use-case return.
pub fn accept<T>(v: T) -> BaseRest<T> {
    Ok(v)
}
