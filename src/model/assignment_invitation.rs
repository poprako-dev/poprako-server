//! Domain models for chapter assignment invitations.

use time::OffsetDateTime;

use crate::value::role::RoleMask;

/// An invitation record for joining a chapter assignment.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentInvitationInfo {
    pub id: String,

    pub chapter_id: String,

    pub inviter_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub pending: bool,

    pub roles: RoleMask,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert an assignment invitation row.
pub struct AssignmentInvitationEntry {
    pub id: String,

    pub chapter_id: String,

    pub inviter_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub roles: RoleMask,
}

/// Filtering and pagination parameters for listing assignment invitations.
pub struct AssignmentInvitationListSpec {
    pub chapter_id: String,
    pub kind: AssignmentInvitationListKind,

    pub offset: u32,
    pub limit: u32,
}

/// Consumption-status filtering mode for listing assignment invitations.
pub enum AssignmentInvitationListKind {
    /// Include invitations regardless of consumption status.
    All,
    /// Include only invitations that have not yet been consumed.
    Pending,
    /// Include only invitations that have already been consumed.
    Used,
}
