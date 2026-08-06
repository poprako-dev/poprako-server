// Shared utility helpers for complex-layer operations.
mod util;
// TODO: Separate pure domain rules from proxy-driven perm checks and
// cascade workflows without changing the current transaction ownership.

/// Announcement business rules and perm checks.
pub mod announcement;
/// Assignment management business rules and perm checks.
pub mod assignment;
/// Chapter lifecycle business rules and perm checks.
pub mod chapter;
/// Chapter port (import/export) business rules and perm checks.
pub mod chapter_port;
/// Comic lifecycle business rules and perm checks.
pub mod comic;
/// Immutable comic archive payload construction.
pub mod comic_archive;
/// Comment business rules and perm checks.
pub mod comment;
/// Image handling business rules and signed URL generation.
pub mod image;
/// Member business rules and perm checks.
pub mod member;
/// Member invitation business rules and perm checks.
pub mod member_invitation;
/// Page business rules and perm checks.
pub mod page;
/// System mail business rules and perm checks.
pub mod system_mail;
/// Team business rules and perm checks.
pub mod team;
/// Terminology-entry construction and validation rules.
pub mod term;
/// Termbase perm checks and cascade rules.
pub mod termbase;
/// Unit ordering, mutation, and perm rules.
pub mod unit;
/// User business rules and perm checks.
pub mod user;
/// Workset lifecycle business rules and perm checks.
pub mod workset;
