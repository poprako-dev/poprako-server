//! Data transfer objects for assignment invitation use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::value::role::RoleMask;

/// Presentation-ready assignment invitation information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AssignmentInvitationInfoVal {
    //
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

impl From<AssignmentInvitationInfo> for AssignmentInvitationInfoVal {
    // Map assignment invitation model fields directly to API-facing values.
    fn from(value: AssignmentInvitationInfo) -> Self {
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

/// Input parameters for listing invitations under one chapter.
///
/// Example: `/api/v1/assignment-invitations?chapter_id=c_1&pending=true&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAssignmentInvitationInfosParams {
    //
    /// Parent chapter whose assignment invitations to list.
    pub chapter_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Input parameters for creating an assignment invitation.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAssignmentInvitationParams {
    //
    /// Identifier of the chapter to create the invitation for.
    pub chapter_id: String,
    /// Qualified identifier of the user being invited.
    pub invitee_qid: String,
    /// Role mask to assign to the invitee upon joining.
    pub roles: RoleMask,
}

/// Return value from creating an assignment invitation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAssignmentInvitationPayload {
    //
    /// Unique identifier of the newly created invitation.
    pub id: String,
    /// Secret invitation code for the invitee to use.
    pub code: String,
}

/// Input parameters for joining an assignment through an invitation code.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct JoinAssignmentInvitationParams {
    /// Secret invitation code to join with.
    pub code: String,
}
