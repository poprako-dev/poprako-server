//! Domain model types representing persisted business entities.
//!
//! Types in this layer carry raw storage values — [`OffsetDateTime`] timestamps,
//! opaque keys, and version numbers. They are converted to presentation-friendly
//! types in the [`data`](super::data) layer before reaching external consumers.

pub mod assignment;
pub mod chapter;
pub mod comic;
pub mod member;
pub mod member_invitation;
pub mod page;
pub mod role;
pub mod system_mail;
pub mod team;
pub mod user;
pub mod workset;
