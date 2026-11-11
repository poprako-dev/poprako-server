//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

#[cfg(not(debug_assertions))]
use veil::Redact;

use crate::api::http::result::Wire;
use crate::data::instr::assignment_invitation::{
    CreateAssignmentInvitationInstr, JoinAssignmentInvitationInstr,
    ListAssignmentInvitationInfosInstr,
};
use crate::data::val::assignment_invitation::CreateAssignmentInvitationVal;
use crate::data::view::assignment_invitation::AssignmentInvitationInfoView;
use crate::value::pagination::PubListLimit;
use crate::value::role::RoleMask;

/// Input parameters for listing invitations under one chapter.
///
/// Example: `/api/v1/assignment-invitations?chapter_id=c_1&is_pending=true&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAssignmentInvitationInfosQuery {
    /// Parent chapter whose assignment invitations to list.
    pub chapter_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListAssignmentInvitationInfosQuery>
    for ListAssignmentInvitationInfosInstr
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListAssignmentInvitationInfosQuery) -> Self {
        Self {
            chapter_id: value.chapter_id,
            is_pending: value.is_pending,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for creating an assignment invitation.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAssignmentInvitationBody {
    /// Identifier of the chapter to create the invitation for.
    pub chapter_id: String,
    /// Qualified identifier of the user being invited.
    pub invitee_qid: String,
    /// Role mask to assign to the invitee upon joining.
    pub roles: RoleMask,
}

impl From<CreateAssignmentInvitationBody> for CreateAssignmentInvitationInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateAssignmentInvitationBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
            invitee_qid: value.invitee_qid,
            roles: value.roles,
        }
    }
}

/// Input parameters for joining an assignment through an invitation code.
#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct JoinAssignmentInvitationBody {
    /// Secret invitation code to join with.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub code: String,
}

impl From<JoinAssignmentInvitationBody> for JoinAssignmentInvitationInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: JoinAssignmentInvitationBody) -> Self {
        Self { code: value.code }
    }
}

/// Return value from creating an assignment invitation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAssignmentInvitationPayload {
    /// Unique identifier of the newly created invitation.
    pub id: String,
    /// Secret invitation code for the invitee to use.
    pub code: String,
}

impl From<CreateAssignmentInvitationVal> for CreateAssignmentInvitationPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateAssignmentInvitationVal) -> Self {
        Self {
            id: value.id,
            code: value.code,
        }
    }
}

impl Wire for CreateAssignmentInvitationVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateAssignmentInvitationPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready assignment invitation information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AssignmentInvitationInfoObj {
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

impl From<AssignmentInvitationInfoView> for AssignmentInvitationInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AssignmentInvitationInfoView) -> Self {
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
            inviter_id: value.inviter_id,
            invitee_qid: value.invitee_qid,
            code: value.code,
            is_pending: value.is_pending,
            roles: value.roles,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for AssignmentInvitationInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = AssignmentInvitationInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query for listing assignment invitations under one chapter.
///
/// Example: `?is_pending=true&offset=0&limit=20`.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct AssignmentInvitationListQuery {
    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
