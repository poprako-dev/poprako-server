//! Convenience type alias for the standard Result, reducing repetition in
//! generic function signatures.

/// A shorthand alias for [`std::result::Result`] to reduce boilerplate in
/// generic return types.
pub type StdResult<T, E> = std::result::Result<T, E>;
