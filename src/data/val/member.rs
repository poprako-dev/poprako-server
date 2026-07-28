//! Val DTOs for the member domain.

//! Data transfer objects for member use cases.

use serde::Serialize;

use crate::data::val::team::TeamInfoVal;
use crate::data::val::user::UserInfoVal;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use futures::future::OptionFuture;

use poprako_util::time::ToUnixMilli;

use crate::model::read::proj::member::MemberInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};
use crate::value::role::RoleMask;

/// Presentation-ready membership information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MemberInfoVal {
    //
    /// Unique member identifier.
    pub id: String,

    /// Owning user identifier.
    pub user_id: String,
    /// Display nickname of the member.
    pub nickname: String,
    /// Unix timestamp of the last activity, in milliseconds.
    pub last_active_at: i64,

    /// Team identifier this membership belongs to.
    pub team_id: String,

    /// Resolved user detail, present when the `user` include option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfoVal>,
    /// Resolved team detail, present when the `team` include option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoVal>,

    /// Bitmask of roles assigned to this member.
    pub roles: RoleMask,
}

impl MemberInfoVal {
    /// Converts a member model into a presentation-ready value,
    /// resolving included user/team instr when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: MemberInfo,
    ) -> BaseRest<Self>
    where
        P: ImagePool,
    {
        accept(Self {
            id: model.id,
            user_id: model.user_id,
            nickname: model.user_nickname,
            last_active_at: model.user_last_active_at.to_unix_milli(),
            team_id: model.team_id,
            user: OptionFuture::from(model.user.map(|user_info| {
                UserInfoVal::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
            team: OptionFuture::from(model.team.map(|team_info| {
                TeamInfoVal::from_model(image_pool, team_info)
            }))
            .await
            .transpose()?,
            roles: model.roles,
        })
    }
}

impl From<MemberInfo> for MemberInfoVal {
    // Convert persisted membership model into response DTO without include expansion.
    fn from(value: MemberInfo) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            nickname: value.user_nickname,
            last_active_at: value.user_last_active_at.to_unix_milli(),
            team_id: value.team_id,
            user: None,
            team: None,
            roles: value.roles,
        }
    }
}

/// Return value from creating a member.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberVal {
    /// Identifier of the created member.
    pub id: String,
}
