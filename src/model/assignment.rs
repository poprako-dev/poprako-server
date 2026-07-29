//! Domain models for chapter assignments — role masks tracking which
//! workflow roles a user holds for a chapter.
//!
//! Each assignment carries a [`RoleMask`] bitmap. The individual
//! workflow stages a user can act on are derived from the mask.
//!
//! Convert to [`AssignmentInfoVal`] for presentation.
//!
//! [`RoleMask`]: crate::value::role::RoleMask
//! [`AssignmentInfoVal`]: crate::data::chapter::AssignmentInfoVal

use time::OffsetDateTime;

use crate::model::chapter::ChapterInfo;
use crate::model::user::UserInfo;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::{RoleField, RoleMask};

/// A chapter assignment record linking a user to a chapter with a set of
/// workflow roles.
///
/// The `roles` mask specifies which workflow roles the user holds for this
/// chapter (translator, proofreader, etc.).
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentInfo {
    //
    /// Unique identifier for the assignment record.
    pub id: String,

    /// Foreign key to the chapter the user is assigned to.
    pub chapter_id: String,
    /// Foreign key to the assigned user.
    pub user_id: String,

    /// Optional joined user data included when the query specifies user expansion.
    pub user: Option<UserInfo>,
    /// Optional joined chapter data included when the query specifies chapter expansion.
    pub chapter: Option<ChapterInfo>,

    /// Bitmask of workflow roles the user holds for this chapter.
    pub roles: RoleMask,

    /// Timestamp when the assignment was created.
    pub created_at: OffsetDateTime,
    /// Timestamp of the last modification to the assignment.
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert a new assignment row.
///
/// The `roles` mask specifies the initial set of roles.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentEntry {
    //
    /// Unique identifier to insert for the new assignment row.
    pub id: String,

    /// Foreign key identifying the chapter whose workflow is joined.
    pub chapter_id: String,
    /// Foreign key identifying the user being added to the chapter workflow.
    pub user_id: String,

    /// Initial bitmask of workflow roles granted to this user.
    pub roles: RoleMask,
}

/// Mutable role fields for a chapter assignment.
///
/// Carries the merged [`RoleMask`] after adding (or removing) roles.
/// The use case layer computes the merge via [`AssignmentComplex::merge_roles`]
/// before constructing this update.
///
/// [`AssignmentComplex::merge_roles`]: crate::complex::assignment::AssignmentComplex::merge_roles
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentRoleUpdate {
    //
    /// Unique identifier of the assignment whose roles are being changed.
    pub id: String,

    /// Updated bitmask of workflow roles after applying the add or remove merge.
    pub roles: RoleMask,
}

/// Filtering and pagination parameters for listing chapter assignments.
pub enum AssignmentInfoListSpec {
    /// List assignments on a specific chapter, optionally filtered by role.
    Chapter {
        //
        /// Foreign key scoping the listing to a single chapter.
        chapter_id: String,
        /// Optional role filter; only assignments with this role in their mask
        /// are returned when set.
        role: Option<RoleField>,
        /// Flags controlling which optional associations are joined into results.
        incl_opt: Vec<AssignmentInclOpt>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: u32,
    },

    /// List assignments owned by a specific user, optionally filtered by role.
    User {
        //
        /// User identifier scoping the listing to assignments owned by this user.
        owner_id: String,
        /// Optional role filter; only assignments with this role in their mask
        /// are returned when set.
        role: Option<RoleField>,
        /// Flags controlling which optional associations are joined into results.
        incl_opt: Vec<AssignmentInclOpt>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: u32,
    },
}
