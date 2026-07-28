//! Domain model types representing persisted business entities.
//!
//! Types in this layer carry raw storage values — [`OffsetDateTime`] timestamps,
//! opaque keys, and version numbers. They are converted to presentation-friendly
//! types in the [`data`](super::data) layer before reaching external consumers.

/// Chapter port persisted entity model.
pub mod chapter_port;
/// Page port persisted entity model.
pub mod page_port;
/// Unit port persisted entity model.
pub mod unit_port;

/// Value groups shared by read and write models.
pub mod shared;

/// Repository read models.
pub mod read;
/// Repository write models.
pub mod write;
