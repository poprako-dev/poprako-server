//! Domain models for chapter assignments — role masks and per-role timestamps
//! tracking when each worker first joined the chapter.
//!
//! Each assignment record carries a [`RoleMask`] bitmap and a set of optional
//! timestamps, one per workflow role. A timestamp is recorded the first time
//! that role is granted, allowing the UI to display "joined as translator on …".
//!
//! Convert to [`AssignmentInfoVal`] for presentation.
//!
//! [`RoleMask`]: crate::value::role::RoleMask
//! [`AssignmentInfoVal`]: crate::data::chapter::AssignmentInfoVal

use time::OffsetDateTime;

use crate::model::role::RoleMask;

/// A chapter assignment record linking a user to a chapter with a set of
/// workflow roles.
///
/// The `*_assigned_at` timestamps are set once when the corresponding
/// [`RoleBit`] first appears in the `role_mask`. They are never cleared
/// when a role is removed, preserving the audit trail.
///
/// [`RoleBit`]: crate::value::role::RoleBit
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentInfo {
    pub id: String,
    pub chapter_id: String,
    pub user_id: String,
    pub role_mask: RoleMask,
    pub raw_provider_assigned_at: Option<OffsetDateTime>,
    pub translator_assigned_at: Option<OffsetDateTime>,
    pub proofreader_assigned_at: Option<OffsetDateTime>,
    pub typesetter_assigned_at: Option<OffsetDateTime>,
    pub redrawer_assigned_at: Option<OffsetDateTime>,
    pub reviewer_assigned_at: Option<OffsetDateTime>,
    pub publisher_assigned_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert a new assignment row.
///
/// The `role_mask` specifies the initial set of roles. Per-role timestamps
/// are derived from the mask at insertion time via
/// [`AssignmentComplex::timed_roles_from_mask`].
///
/// [`AssignmentComplex::timed_roles_from_mask`]: crate::complex::assignment::AssignmentComplex::timed_roles_from_mask
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentForm {
    pub id: String,
    pub chapter_id: String,
    pub user_id: String,
    pub role_mask: RoleMask,
}

/// Mutable role fields for a chapter assignment.
///
/// Carries the merged [`RoleMask`] after adding (or removing) roles, along
/// with the updated per-role timestamps. The use case layer computes the
/// merge via [`AssignmentComplex::merge_timed_roles`] before constructing
/// this update.
///
/// [`AssignmentComplex::merge_timed_roles`]: crate::complex::assignment::AssignmentComplex::merge_timed_roles
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentRoleUpdate {
    pub id: String,
    pub role_mask: RoleMask,
    pub raw_provider_assigned_at: Option<OffsetDateTime>,
    pub translator_assigned_at: Option<OffsetDateTime>,
    pub proofreader_assigned_at: Option<OffsetDateTime>,
    pub typesetter_assigned_at: Option<OffsetDateTime>,
    pub redrawer_assigned_at: Option<OffsetDateTime>,
    pub reviewer_assigned_at: Option<OffsetDateTime>,
    pub publisher_assigned_at: Option<OffsetDateTime>,
}
