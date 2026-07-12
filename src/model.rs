//! Domain model types representing persisted business entities.
//!
//! Types in this layer carry raw storage values — [`OffsetDateTime`] timestamps,
//! opaque keys, and version numbers. They are converted to presentation-friendly
//! types in the [`data`](super::data) layer before reaching external consumers.

/// Announcement persisted entity model.
mod announcement;
/// Assignment persisted entity model.
mod assignment;
/// Assignment invitation persisted entity model.
mod assignment_invitation;
/// Chapter persisted entity model.
mod chapter;
/// Chapter port persisted entity model.
mod chapter_port;
/// Comic persisted entity model.
mod comic;
/// Immutable comic archive snapshots and persisted records.
mod comic_archive;
/// Comment persisted entity model.
mod comment;
/// Member persisted entity model.
mod member;
/// Member invitation persisted entity model.
mod member_invitation;
/// Page persisted entity model.
mod page;
/// Page port persisted entity model.
mod page_port;
/// System mail persisted entity model.
mod system_mail;
/// Team persisted entity model.
mod team;
/// Unit persisted entity model.
mod unit;
/// Unit port persisted entity model.
mod unit_port;
/// User persisted entity model.
mod user;
/// Workset persisted entity model.
mod workset;

pub mod announcement_model {
    pub use crate::model::announcement::*;
}
pub mod assignment_model {
    pub use crate::model::assignment::*;
}
pub mod assignment_invitation_model {
    pub use crate::model::assignment_invitation::*;
}
pub mod chapter_model {
    pub use crate::model::chapter::*;
}
pub mod chapter_port_model {
    pub use crate::model::chapter_port::*;
}
pub mod comic_model {
    pub use crate::model::comic::*;
}
pub mod comic_archive_model {
    pub use crate::model::comic_archive::*;
}
pub mod comment_model {
    pub use crate::model::comment::*;
}
pub mod member_model {
    pub use crate::model::member::*;
}
pub mod member_invitation_model {
    pub use crate::model::member_invitation::*;
}
pub mod page_model {
    pub use crate::model::page::*;
}
pub mod page_port_model {
    pub use crate::model::page_port::*;
}
pub mod system_mail_model {
    pub use crate::model::system_mail::*;
}
pub mod team_model {
    pub use crate::model::team::*;
}
pub mod unit_model {
    pub use crate::model::unit::*;
}
pub mod unit_port_model {
    pub use crate::model::unit_port::*;
}
pub mod user_model {
    pub use crate::model::user::*;
}
pub mod workset_model {
    pub use crate::model::workset::*;
}
