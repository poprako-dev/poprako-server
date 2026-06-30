//! Data transfer objects for assignment invitation use cases.

use poprako_util::time::ToUnixMilli;

use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::model::role::RoleMask;

/// Presentation-ready assignment invitation information.
pub struct AssignmentInvitationInfoVal {
    pub id: String,

    pub chapter_id: String,

    pub inviter_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub pending: bool,

    pub roles: RoleMask,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<AssignmentInvitationInfo> for AssignmentInvitationInfoVal {
    fn from(value: AssignmentInvitationInfo) -> Self {
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
            inviter_id: value.inviter_id,
            invitee_qid: value.invitee_qid,
            code: value.code,
            pending: value.pending,
            roles: value.roles,
            created_at: value.created_at.to_unix_milli(),
            updated_at: value.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for listing invitations under one chapter.
pub struct ListAssignmentInvitationInfosData {
    pub chapter_id: String,
    pub pending: Option<bool>,
    pub offset: u64,
    pub limit: u64,
}

/// Input parameters for creating an assignment invitation.
pub struct CreateAssignmentInvitationData {
    pub chapter_id: String,
    pub invitee_qid: String,
    pub roles: RoleMask,
}

/// Return value from creating an assignment invitation.
pub struct CreateAssignmentInvitationVal {
    pub id: String,
    pub code: String,
}

/// Input parameters for joining an assignment through an invitation code.
pub struct JoinAssignmentInvitationData {
    pub code: String,
}
