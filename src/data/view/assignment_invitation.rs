//! View DTOs for the assignment-invitation domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::value::role::RoleMask;

/// Presentation-ready assignment invitation information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AssignmentInvitationInfoView {
    /// Unique identifier of the invitation.
    pub id: String,

    /// Identifier of the chapter this invitation belongs to.
    pub chapter_id: String,

    /// User identifier of the inviter who created this invitation.
    pub inviter_id: String,
    /// Qualified identifier of the user being invited.
    pub invitee_qid: String,

    /// Secret invitation code used for joining.
    pub code: String,

    /// Whether the invitation has not yet been consumed.
    pub is_pending: bool,

    /// Role mask assigned to the invitation.
    pub roles: RoleMask,

    /// Timestamp of creation in milliseconds.
    pub created_at: i64,
    /// Timestamp of last update in milliseconds.
    pub updated_at: i64,
}

impl From<AssignmentInvitationInfo> for AssignmentInvitationInfoView {
    // Map assignment invitation model fields directly to API-facing values.
    fn from(value: AssignmentInvitationInfo) -> Self {
        //
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
            inviter_id: value.inviter_id,
            invitee_qid: value.invitee_qid,
            code: value.code,
            is_pending: value.is_pending,
            roles: value.roles,
            created_at: value.created_at.to_unix_milli(),
            updated_at: value.updated_at.to_unix_milli(),
        }
    }
}
