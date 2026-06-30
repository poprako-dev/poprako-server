//! Domain models for chapter assignment invitations.

use time::OffsetDateTime;

use crate::model::role::RoleMask;

/// An invitation record for joining a chapter assignment.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentInvitationInfo {
    pub id: String,

    pub chapter_id: String,

    pub inviter_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub pending: bool,

    pub role_mask: RoleMask,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert an assignment invitation row.
pub struct AssignmentInvitationForm {
    pub id: String,

    pub chapter_id: String,

    pub inviter_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub role_mask: RoleMask,
}
