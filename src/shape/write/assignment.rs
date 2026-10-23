//! Borrowed assignment write payloads.
//! Owners remain alive through persistence; existing data is not copied.

use crate::value::role::RoleMask;

/// The data needed to insert a new assignment row.
///
/// The `roles` mask specifies the initial roles.
/// Existing foreign identifiers may borrow their caller-owned storage.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentEntryShape<'a> {
    /// Unique identifier to insert for the new assignment row.
    pub id: String,

    /// Foreign key identifying the chapter whose workflow is joined.
    pub chapter_id: &'a str,
    /// Foreign key identifying the user being added to the chapter workflow.
    pub user_id: &'a str,

    /// Initial bitmask of workflow roles granted to this user.
    pub roles: RoleMask,
}
