#![allow(clippy::option_option)]
#![allow(clippy::struct_field_names)]

//! Diesel entity types for the RDB repository.

/// Announcement Diesel entity types.
pub mod announcement;
/// Assignment Diesel entity types.
pub mod assignment;
/// Assignment invitation Diesel entity types.
pub mod assignment_invitation;
/// Chapter Diesel entity types.
pub mod chapter;
/// Immutable chapter workflow record Diesel entity types.
pub mod chapter_workflow_record;
/// Comic Diesel entity types.
pub mod comic;
/// Immutable comic archive Diesel entity types.
pub mod comic_archive;
/// Comment Diesel entity types.
pub mod comment;
/// Member Diesel entity types.
pub mod member;
/// Member invitation Diesel entity types.
pub mod member_invitation;
/// Page Diesel entity types.
pub mod page;
/// System mail Diesel entity types.
pub mod system_mail;
/// Team Diesel entity types.
pub mod team;
/// Term Diesel entity types.
pub mod term;
/// Termbase Diesel entity types.
pub mod termbase;
/// Unit Diesel entity types.
pub mod unit;
/// User Diesel entity types.
pub mod user;
/// Workset Diesel entity types.
pub mod workset;
