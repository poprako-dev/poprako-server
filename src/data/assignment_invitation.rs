//! Data transfer objects for assignment invitation use cases.

use serde::{Deserialize, Serialize};

use utoipa::{IntoParams, ToSchema};

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::value::role::RoleMask;

/// Presentation-ready assignment invitation information.
#[derive(Debug, Serialize, ToSchema)]
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
///
/// Example: `/api/v1/assignment-invitations?chapter_id=c_1&pending=true&offset=0&limit=20`.
#[Paginate]
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListAssignmentInvitationInfosData {
    /// Parent chapter whose assignment invitations to list.
    pub chapter_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub pending: Option<bool>,
}

/// Input parameters for creating an assignment invitation.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAssignmentInvitationData {
    pub chapter_id: String,
    pub invitee_qid: String,
    pub roles: RoleMask,
}

/// Return value from creating an assignment invitation.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateAssignmentInvitationVal {
    pub id: String,
    pub code: String,
}

/// Input parameters for joining an assignment through an invitation code.
#[derive(Debug, Deserialize, ToSchema)]
pub struct JoinAssignmentInvitationData {
    pub code: String,
}
