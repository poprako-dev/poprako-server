//! Diesel-backed operations for the database repository adapter.
// Submit query macro that allocates a connection and calls a free function.
#[macro_use]
mod submit_query;
// Numeric conversions at the database boundary.
mod numeric;
// Announcement repository operations.
mod announcement;
// Assignment repository operations.
mod assignment;
// Assignment invitation repository operations.
mod assignment_invitation;
// Chapter repository operations.
mod chapter;
// Immutable chapter workflow record repository operations.
mod chapter_workflow_record;
// Comic repository operations.
mod comic;
// Immutable comic archive repository operations.
mod comic_archive;
// Comment repository operations.
mod comment;
// Member repository operations.
mod member;
// Member invitation repository operations.
mod member_invitation;
// Composite artwork pages.
mod page_artwork;
// source page repository operations.
mod page_source;
// Hierarchical subtree deletion operations.
mod subtree_delete;
// System mail repository operations.
mod system_mail;
// Team repository operations.
mod team;
// Term repository operations.
mod term;
// Termbase repository operations.
mod termbase;
// Current chapter review issues.
mod issue;
// Versioned review editing.
mod issue_edit;
// Unit repository operations.
mod unit;
// User repository operations.
mod user;
// Workset repository operations.
mod workset;

/// Entity types for the RDB repository.
pub mod entity;
/// Batch include helpers.
pub mod incl;
/// Diesel-generated schema.
pub mod schema;

/// Shared RDB integration-test fixtures.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod test_shared;

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
mod tests;
