//! Domain model types representing persisted business entities.
//!
//! Types in this layer carry raw storage values — [`OffsetDateTime`] timestamps,
//! opaque keys, and version numbers. They are converted to presentation-friendly
//! types in the [`data`](super::data) layer before reaching external consumers.

/// Announcement persisted entity model.
pub mod announcement;
/// Assignment persisted entity model.
pub mod assignment;
/// Assignment invitation persisted entity model.
pub mod assignment_invitation;
/// Chapter persisted entity model.
pub mod chapter;
/// Chapter port persisted entity model.
pub mod chapter_port;
/// Comic persisted entity model.
pub mod comic;
/// Comment persisted entity model.
pub mod comment;
/// Member persisted entity model.
pub mod member;
/// Member invitation persisted entity model.
pub mod member_invitation;
/// Page persisted entity model.
pub mod page;
/// Page port persisted entity model.
pub mod page_port;
/// System mail persisted entity model.
pub mod system_mail;
/// Team persisted entity model.
pub mod team;
/// Unit persisted entity model.
pub mod unit;
/// Unit port persisted entity model.
pub mod unit_port;
/// User persisted entity model.
pub mod user;
/// Workset persisted entity model.
pub mod workset;
