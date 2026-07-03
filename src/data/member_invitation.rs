//! Data transfer objects for member invitation use cases — input parameters
//! and presentation-ready invitation values.

use serde::{Deserialize, Serialize};

use utoipa::{IntoParams, ToSchema};

use poprako_macro::Paginate;

use crate::data::user::UserInfoVal;
use crate::model::member_invitation::MemberInvitationInfo;
use crate::part::image::ImagePool;
use crate::result::RegularResult;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

/// Input parameters for creating a new team invitation.
///
/// The invitation binds a specific QQ ID (`invitee_qid`) to a [`RoleMask`]
/// that will be granted upon acceptance. The actual in-app user lookup
/// happens during the registration flow.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMemberInvitationData {
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
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateMemberInvitationVal {
    pub id: String,
    pub code: String,
}

/// Input parameters for listing invitations within a team, with optional
/// pending-status filtering and standard offset/limit pagination.
#[Paginate]
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListMemberInvitationInfosData {
    pub team_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub pending: Option<bool>,

    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<MemberInvitationInclOpt>,
}

/// Presentation-ready member invitation information.
///
/// Mirrors [`MemberInvitationInfo`] with timestamps omitted (the domain
/// model carries no timestamps).
///
/// [`MemberInvitationInfo`]: crate::model::member_invitation::MemberInvitationInfo
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberInvitationInfoVal {
    pub id: String,

    pub team_id: String,

    pub invitor_id: String,
    pub invitor: Option<UserInfoVal>,

    pub invitee_qid: String,
    pub code: String,

    pub pending: bool,

    pub roles: RoleMask,
}

impl From<MemberInvitationInfo> for MemberInvitationInfoVal {
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

impl MemberInvitationInfoVal {
    /// Converts an invitation model into a presentation-ready value,
    /// resolving included invitor avatar when present.
    pub async fn from_model<P>(image_pool: &P, model: MemberInvitationInfo) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let invitor = match model.invitor {
            Some(user_info) => Some(UserInfoVal::from_model(image_pool, user_info).await?),
            None => None,
        };

        Ok(Self {
            id: model.id,
            team_id: model.team_id,
            invitor_id: model.invitor_id,
            invitor,
            invitee_qid: model.invitee_qid,
            code: model.code,
            pending: model.pending,
            roles: model.roles,
        })
    }
}

/// Input parameters for updating a pending invitation's roles.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMemberInvitationInfoData {
    pub id: String,
    pub roles: RoleMask,
}
