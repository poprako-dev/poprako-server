//! Instr DTOs for the member invitation domain.

//! Data transfer objects for member invitation use cases — input parameters
//! and presentation-ready invitation values.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

/// Input parameters for creating a new team invitation.
///
/// The invitation binds a specific QQ ID (`invitee_qid`) to a [`RoleMask`]
/// that will be granted upon acceptance. The actual in-app user lookup
/// happens during the registration flow.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberInvitationInstr {
    /// Owning team identifier.
    pub team_id: String,

    /// The QQ ID of the person being invited (not a user UUID).
    pub invitee_qid: String,

    /// The role mask that will be assigned when the invitee registers
    /// and accepts the invitation.
    pub roles: RoleMask,
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
pub struct ListMemberInvitationInfosInstr {
    /// Parent team whose invitations to list.
    pub team_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Related rows to embed. Repeatable. Values: `invitor`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<MemberInvitationInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Input parameters for updating a pending invitation's roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateMemberInvitationRolesInstr {
    /// Invitation identifier.
    pub id: String,
    /// New role mask for the invitation.
    pub roles: RoleMask,
}
