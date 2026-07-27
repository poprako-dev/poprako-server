//! Data transfer objects for member invitation use cases — input parameters
//! and presentation-ready invitation values.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use futures::future::OptionFuture;

use crate::data::user::UserInfoVal;
use crate::model::member_invitation::MemberInvitationInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

/// Input parameters for creating a new team invitation.
///
/// The invitation binds a specific QQ ID (`invitee_qid`) to a [`RoleMask`]
/// that will be granted upon acceptance. The actual in-app user lookup
/// happens during the registration flow.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberInvitationParams {
    //
    /// Owning team identifier.
    pub team_id: String,

    /// The QQ ID of the person being invited (not a user UUID).
    pub invitee_qid: String,

    /// The role mask that will be assigned when the invitee registers
    /// and accepts the invitation.
    pub roles: RoleMask,
}

/// Return value from a successful invitation creation.
///
/// The `code` is a short opaque token the invitee presents during
/// registration to claim the invitation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberInvitationPayload {
    //
    /// Unique identifier of the created invitation.
    pub id: String,
    /// Opaque invitation code presented by the invitee to claim the invitation.
    pub code: String,
}

/// Input parameters for listing invitations within a team, with optional
/// pending-status filtering and standard offset/limit pagination.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/teams/{team_id}/member-invitations?pending=true&incl=invitor&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListMemberInvitationInfosParams {
    //
    /// Parent team whose invitations to list.
    pub team_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub pending: Option<bool>,

    /// Related rows to embed. Repeatable. Values: `invitor`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<MemberInvitationInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Presentation-ready member invitation information.
///
/// Mirrors [`MemberInvitationInfo`] with timestamps omitted (the domain
/// model carries no timestamps).
///
/// [`MemberInvitationInfo`]: crate::model::member_invitation::MemberInvitationInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MemberInvitationInfoVal {
    //
    /// Unique identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,

    /// Identifier of the user who created the invitation.
    pub invitor_id: String,
    /// Resolved invitor user information, present only when requested via incl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitor: Option<UserInfoVal>,

    /// QQ ID of the person being invited.
    pub invitee_qid: String,
    /// Opaque invitation code.
    pub code: String,

    /// Whether the invitation has not yet been consumed.
    pub pending: bool,

    /// Role mask assigned to the invitee upon acceptance.
    pub roles: RoleMask,
}

impl MemberInvitationInfoVal {
    /// Converts an invitation model into a presentation-ready value,
    /// resolving included invitor avatar when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: MemberInvitationInfo,
    ) -> BaseRest<Self>
    where
        P: ImagePool,
    {
        accept(Self {
            id: model.id,
            team_id: model.team_id,
            invitor_id: model.invitor_id,
            invitor: OptionFuture::from(model.invitor.map(|user_info| {
                UserInfoVal::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
            invitee_qid: model.invitee_qid,
            code: model.code,
            pending: model.pending,
            roles: model.roles,
        })
    }
}

impl From<MemberInvitationInfo> for MemberInvitationInfoVal {
    // Convert invitation model into response value without preloaded invitor payload.
    fn from(value: MemberInvitationInfo) -> Self {
        Self {
            id: value.id,
            team_id: value.team_id,
            invitor_id: value.invitor_id,
            invitor: None,
            invitee_qid: value.invitee_qid,
            code: value.code,
            pending: value.pending,
            roles: value.roles,
        }
    }
}

/// Input parameters for updating a pending invitation's roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateMemberInvitationRolesParams {
    //
    /// Invitation identifier.
    pub id: String,
    /// New role mask for the invitation.
    pub roles: RoleMask,
}
