//! Data transfer objects that sit between the external world and use cases.
//!
//! Types suffixed with `Data` carry inbound request payloads. Types suffixed
//! with `Val` carry presentation-ready outbound values — timestamps are
//! converted to Unix milliseconds, and avatar URLs are resolved through
//! [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

/// Announcement request/response DTOs.
mod announcement;
/// Assignment request/response DTOs.
mod assignment;
/// Assignment invitation request/response DTOs.
mod assignment_invitation;
/// Authentication request DTOs.
mod auth;
/// Chapter request/response DTOs.
mod chapter;
/// Chapter port (import/export) request/response DTOs.
mod chapter_port;
/// Comic request/response DTOs.
mod comic;
/// Immutable comic archive response DTOs.
mod comic_archive;
/// Comment request/response DTOs.
mod comment;
/// Member request/response DTOs.
mod member;
/// Member invitation request/response DTOs.
mod member_invitation;
/// Page request/response DTOs.
mod page;
/// Page port DTOs.
mod page_port;
/// System mail request/response DTOs.
mod system_mail;
/// Team request/response DTOs.
mod team;
/// Unit request/response DTOs.
mod unit;
/// Unit port DTOs.
mod unit_port;
/// User request/response DTOs.
mod user;
/// Workset request/response DTOs.
mod workset;

pub mod announcement_data {
    pub use crate::data::announcement::*;
}
pub mod assignment_data {
    pub use crate::data::assignment::*;
}
pub mod assignment_invitation_data {
    pub use crate::data::assignment_invitation::*;
}
pub mod auth_data {
    pub use crate::data::auth::*;
}
pub mod chapter_data {
    pub use crate::data::chapter::*;
}
pub mod chapter_port_data {
    pub use crate::data::chapter_port::*;
}
pub mod comic_data {
    pub use crate::data::comic::*;
}
pub mod comic_archive_data {
    pub use crate::data::comic_archive::*;
}
pub mod comment_data {
    pub use crate::data::comment::*;
}
pub mod member_data {
    pub use crate::data::member::*;
}
pub mod member_invitation_data {
    pub use crate::data::member_invitation::*;
}
pub mod page_data {
    pub use crate::data::page::*;
}
pub mod page_port_data {
    pub use crate::data::page_port::*;
}
pub mod system_mail_data {
    pub use crate::data::system_mail::*;
}
pub mod team_data {
    pub use crate::data::team::*;
}
pub mod unit_data {
    pub use crate::data::unit::*;
}
pub mod unit_port_data {
    pub use crate::data::unit_port::*;
}
pub mod user_data {
    pub use crate::data::user::*;
}
pub mod workset_data {
    pub use crate::data::workset::*;
}
