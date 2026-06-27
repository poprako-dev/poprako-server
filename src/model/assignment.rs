//! Domain models for chapter assignments.

use time::OffsetDateTime;

use crate::model::role::RoleMask;

/// A chapter assignment record.
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

/// The data needed to create a chapter assignment.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentForm {
    pub id: String,
    pub chapter_id: String,
    pub user_id: String,
    pub role_mask: RoleMask,
}

/// Mutable role fields for a chapter assignment.
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
