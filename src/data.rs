//! Data transfer objects that sit between the external world and use cases.
//!
//! Types suffixed with `Params` carry use-case input, while types suffixed
//! with `Payload` carry use-case output. `Val` is reserved for serde-facing
//! representations converted from domain models — timestamps are converted
//! to Unix milliseconds, and image keys are resolved through [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

/// Announcement request/response DTOs.
pub mod announcement;
/// Assignment request/response DTOs.
pub mod assignment;
/// Assignment invitation request/response DTOs.
pub mod assignment_invitation;
/// Authentication request DTOs.
pub mod auth;
/// Chapter request/response DTOs.
pub mod chapter;
/// Chapter port (import/export) request/response DTOs.
pub mod chapter_port;
/// Comic request/response DTOs.
pub mod comic;
/// Immutable comic archive response DTOs.
pub mod comic_archive;
/// Comic list endpoint payload.
pub mod comic_list;
/// Comment request/response DTOs.
pub mod comment;
/// Shared image-upload request and response DTOs.
pub mod image;
/// Member request/response DTOs.
pub mod member;
/// Member invitation request/response DTOs.
pub mod member_invitation;
/// Page request/response DTOs.
pub mod page;
/// Page port DTOs.
pub mod page_port;
/// System mail request/response DTOs.
pub mod system_mail;
/// Team request/response DTOs.
pub mod team;
/// Term request and response DTOs.
pub mod term;
/// Termbase request and response data types.
pub mod termbase;
/// Unit request/response DTOs.
pub mod unit;
/// Unit port DTOs.
pub mod unit_port;
/// User request/response DTOs.
pub mod user;
/// Workset request/response DTOs.
pub mod workset;
