//! Data transfer objects that sit between the external world and use cases.
//!
//! Types suffixed with `Data` carry inbound request payloads. Types suffixed
//! with `Val` carry presentation-ready outbound values — timestamps are
//! converted to Unix milliseconds, and avatar URLs are resolved through
//! [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

// FIXME: grouping
pub mod announcement;
pub mod assignment;
pub mod assignment_invitation;
pub mod auth;
pub mod chapter;
pub mod chapter_port;
pub mod comic;
pub mod comment;
pub mod member;
pub mod member_invitation;
pub mod page;
pub mod system_mail;
pub mod team;
pub mod unit;
pub mod user;
pub mod workset;
