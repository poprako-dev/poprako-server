/// Shared utility helpers for complex-layer operations.
mod util;

/// Announcement business rules and permission checks.
pub mod announcement;
/// Comment business rules and permission checks.
pub mod comment;
/// Member business rules and permission checks.
pub mod member;
/// Member invitation business rules and permission checks.
pub mod member_invitation;
/// System mail business rules and permission checks.
pub mod system_mail;
/// Team business rules and permission checks.
pub mod team;
/// User business rules and permission checks.
pub mod user;

/// Assignment management business rules and permission checks.
pub mod assignment;
/// Chapter lifecycle business rules and permission checks.
pub mod chapter;
/// Comic lifecycle business rules and permission checks.
pub mod comic;
/// Immutable comic archive payload construction.
pub mod comic_archive;
/// Image handling business rules and signed URL generation.
pub mod image;
/// Page business rules and permission checks.
pub mod page;
/// Unit ordering and diff business rules.
pub mod unit;
/// Workset lifecycle business rules and permission checks.
pub mod workset;

/// Chapter port (import/export) business rules and permission checks.
pub mod chapter_port;
