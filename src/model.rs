//! Domain model types representing persisted business entities.
//!
//! Types in this layer carry raw storage values — [`OffsetDateTime`] timestamps,
//! opaque keys, and version numbers. They are converted to presentation-friendly
//! types in the [`data`](super::data) layer before reaching external consumers.

/// Intermediate artifacts produced while processing domain inputs.
pub mod artifact;
/// Repository read models.
pub mod read;
/// Value groups shared by read and write models.
pub mod shared;
/// Repository write models.
pub mod write;
