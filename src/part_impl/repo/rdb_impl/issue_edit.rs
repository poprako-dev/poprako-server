//! Typed transactional review persistence.

// Typed document reads.
mod read;
// Authorizes a chapter write.
mod write;

/// Transactional revision operations.
pub mod revision;
