//! View DTOs for the member-invitation domain.

use serde::Serialize;

use crate::data::view::user::UserInfoView;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use futures::future::OptionFuture;

use crate::model::read::proj::member_invitation::MemberInvitationInfo;

use crate::part::image::ImagePool;

use crate::result::{BaseRest, accept};

use crate::value::role::RoleMask;

/// Presentation-ready member invitation information.
///
/// Mirrors [`MemberInvitationInfo`] with timestamps omitted (the domain
/// model carries no timestamps).
///
/// [`MemberInvitationInfo`]: crate::model::read::proj::member_invitation::MemberInvitationInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MemberInvitationInfoView {
    /// Unique identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,

    /// Identifier of the user who created the invitation.
    pub invitor_id: String,
    /// Resolved invitor user information, present only when requested via incl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitor: Option<UserInfoView>,

    /// QQ ID of the person being invited.
    pub invitee_qid: String,
    /// Opaque invitation code.
    pub code: String,

    /// Whether the invitation has not yet been consumed.
    pub is_pending: bool,

    /// Role mask assigned to the invitee upon acceptance.
    pub roles: RoleMask,
}

impl MemberInvitationInfoView {
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
                UserInfoView::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
            invitee_qid: model.invitee_qid,
            code: model.code,
            is_pending: model.is_pending,
            roles: model.roles,
        })
    }
}

impl From<MemberInvitationInfo> for MemberInvitationInfoView {
    // Convert invitation model into response value without preloaded invitor payload.
    fn from(value: MemberInvitationInfo) -> Self {
        //
        Self {
            id: value.id,
            team_id: value.team_id,
            invitor_id: value.invitor_id,
            invitor: None,
            invitee_qid: value.invitee_qid,
            code: value.code,
            is_pending: value.is_pending,
            roles: value.roles,
        }
    }
}
