//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::object::user::UserInfoObj;
use crate::api::http::result::Wire;
use crate::data::instr::member_invitation::{
    CreateMemberInvitationInstr, ListMemberInvitationInfosInstr,
    UpdateMemberInvitationRolesInstr,
};
use crate::data::val::member_invitation::CreateMemberInvitationVal;
use crate::data::view::member_invitation::MemberInvitationInfoView;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::pagination::PubListLimit;
use crate::value::role::RoleMask;

/// Input parameters for creating a new team invitation.
///
/// The invitation binds a specific QQ ID (`invitee_qid`) to a [`RoleMask`]
/// that will be granted upon acceptance. The actual in-app user lookup
/// happens during the registration flow.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberInvitationBody {
    /// Owning team identifier.
    pub team_id: String,

    /// The QQ ID of the person being invited (not a user UUID).
    pub invitee_qid: String,

    /// The role mask that will be assigned when the invitee registers
    /// and accepts the invitation.
    pub roles: RoleMask,
}

impl From<CreateMemberInvitationBody> for CreateMemberInvitationInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateMemberInvitationBody) -> Self {
        Self {
            team_id: value.team_id,
            invitee_qid: value.invitee_qid,
            roles: value.roles,
        }
    }
}

/// Input parameters for listing invitations within a team, with optional
/// pending-status filtering and standard offset/limit pagination.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/teams/{team_id}/member-invitations?is_pending=true&incl=invitor&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListMemberInvitationInfosQuery {
    /// Parent team whose invitations to list.
    pub team_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Related rows to embed. Repeatable. Values: `invitor`.
    #[serde(default)]
    pub incl: Vec<MemberInvitationInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

impl From<ListMemberInvitationInfosQuery> for ListMemberInvitationInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListMemberInvitationInfosQuery) -> Self {
        Self {
            team_id: value.team_id,
            is_pending: value.is_pending,
            incl: value.incl,
            offset: value.offset,
            limit: value.limit,
        }
    }
}

/// Input parameters for updating a pending invitation's roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateMemberInvitationRolesBody {
    /// Invitation identifier.
    pub id: String,
    /// New role mask for the invitation.
    pub roles: RoleMask,
}

impl From<UpdateMemberInvitationRolesBody>
    for UpdateMemberInvitationRolesInstr
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: UpdateMemberInvitationRolesBody) -> Self {
        Self {
            id: value.id,
            roles: value.roles,
        }
    }
}

/// Return value from a successful invitation creation.
///
/// The `code` is a short opaque token the invitee presents during
/// registration to claim the invitation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberInvitationPayload {
    /// Unique identifier of the created invitation.
    pub id: String,
    /// Opaque invitation code presented by the invitee to claim the invitation.
    pub code: String,
}

impl From<CreateMemberInvitationVal> for CreateMemberInvitationPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreateMemberInvitationVal) -> Self {
        Self {
            id: value.id,
            code: value.code,
        }
    }
}

impl Wire for CreateMemberInvitationVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreateMemberInvitationPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready member invitation information.
///
/// Mirrors [`MemberInvitationInfo`] with timestamps omitted (the domain
/// model carries no timestamps).
///
/// [`MemberInvitationInfo`]: crate::model::read::proj::member_invitation::MemberInvitationInfo
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MemberInvitationInfoObj {
    /// Unique identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,

    /// Identifier of the user who created the invitation.
    pub invitor_id: String,
    /// Resolved invitor user information, present only when requested via incl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitor: Option<UserInfoObj>,

    /// QQ ID of the person being invited.
    pub invitee_qid: String,
    /// Opaque invitation code.
    pub code: String,

    /// Whether the invitation has not yet been consumed.
    pub is_pending: bool,

    /// Role mask assigned to the invitee upon acceptance.
    pub roles: RoleMask,
}

impl From<MemberInvitationInfoView> for MemberInvitationInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: MemberInvitationInfoView) -> Self {
        Self {
            id: value.id,
            team_id: value.team_id,
            invitor_id: value.invitor_id,
            invitor: value.invitor.map(std::convert::Into::into),
            invitee_qid: value.invitee_qid,
            code: value.code,
            is_pending: value.is_pending,
            roles: value.roles,
        }
    }
}

impl Wire for MemberInvitationInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = MemberInvitationInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query for listing invitations within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?is_pending=true&incl=invitor&offset=0&limit=20`.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct MemberInvitationListQuery {
    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Related rows to embed. Repeatable. Values: `invitor`.
    #[serde(default)]
    pub incl: Vec<MemberInvitationInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}
