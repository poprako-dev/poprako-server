//! Domain model types representing persisted business entities.
//!
//! Types in this layer carry raw storage values — [`OffsetDateTime`] timestamps,
//! opaque keys, and version numbers. They are converted to presentation-friendly
//! types in the [`data`](super::data) layer before reaching external consumers.

/// Page port persisted entity model.
pub mod page_port;
/// Repository read models.
pub mod read;
/// Value groups shared by read and write models.
pub mod shared;
/// Unit port persisted entity model.
pub mod unit_port;
/// Repository write models.
pub mod write;
