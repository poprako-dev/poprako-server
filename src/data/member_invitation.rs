//! Data transfer objects for member invitation use cases — input parameters
//! and presentation-ready invitation values.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_macro::Paginate;

use crate::data::user_data;
use crate::model::member_invitation_model;
use crate::part::image::ImagePool;
use crate::result::RegularResult;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

/// Input parameters for creating a new team invitation.
///
/// The invitation binds a specific QQ ID (`invitee_qid`) to a [`RoleMask`]
/// that will be granted upon acceptance. The actual in-app user lookup
/// happens during the registration flow.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateData {
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
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateVal {
    pub id: String,
    pub code: String,
}

/// Input parameters for listing invitations within a team, with optional
/// pending-status filtering and standard offset/limit pagination.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/teams/{team_id}/member-invitations?pending=true&incl=invitor&offset=0&limit=20`.
#[Paginate]
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListInfosData {
    /// Parent team whose invitations to list.
    pub team_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub pending: Option<bool>,

    /// Related rows to embed. Repeatable. Values: `invitor`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<MemberInvitationInclOpt>,
}

/// Presentation-ready member invitation information.
///
/// Mirrors [`MemberInvitationInfo`] with timestamps omitted (the domain
/// model carries no timestamps).
///
/// [`MemberInvitationInfo`]: crate::model::member_invitation::MemberInvitationInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct InfoVal {
    pub id: String,

    pub team_id: String,

    pub invitor_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitor: Option<user_data::InfoVal>,

    pub invitee_qid: String,
    pub code: String,

    pub pending: bool,

    pub roles: RoleMask,
}

impl From<member_invitation_model::Info> for InfoVal {
    fn from(value: member_invitation_model::Info) -> Self {
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

impl InfoVal {
    /// Converts an invitation model into a presentation-ready value,
    /// resolving included invitor avatar when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: member_invitation_model::Info,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let invitor = match model.invitor {
            //
            Some(user_info) => Some(
                user_data::InfoVal::from_model(image_pool, user_info).await?,
            ),

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
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateRolesData {
    pub id: String,
    pub roles: RoleMask,
}
