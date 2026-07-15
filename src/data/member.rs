//! Data transfer objects for member use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use futures::future::OptionFuture;

use poprako_util::i18n::trl;
use poprako_util::time::ToUnixMilli;

use crate::data::team::TeamInfoVal;
use crate::data::user::UserInfoVal;
use crate::model::member::{MemberInfo, MemberListSpec};
use crate::part::image::ImagePool;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

/// Presentation-ready membership information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct MemberInfoVal {
    pub id: String,

    pub user_id: String,
    pub nickname: String,
    pub last_active_at: i64,

    pub team_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfoVal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoVal>,

    pub roles: RoleMask,
}

impl From<MemberInfo> for MemberInfoVal {
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

impl MemberInfoVal {
    /// Converts a member model into a presentation-ready value,
    /// resolving included user/team params when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: MemberInfo,
    ) -> BaseResult<Self>
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

/// Input parameters for creating a member.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateMemberParams {
    pub user_id: String,
    pub team_id: String,

    pub roles: RoleMask,
}

/// Return value from creating a member.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateMemberPayload {
    pub id: String,
}

/// Input parameters for joining a team through a member invitation.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct JoinTeamParams {
    pub code: String,
}

/// Input parameters for listing members by team.
///
/// Exactly one of `owner_id` or `team_id` is required:
/// - `owner_id`: list teams/memberships owned by that user; `role` and
///   `fuzzy_nickname` must be omitted in this mode;
/// - `team_id`: list members of that team, optionally narrowed by
///   `fuzzy_nickname` and/or `role`.
///
/// `incl` embeds related rows (`user`, `team`).
///
/// Example: `/api/v1/members?team_id=t_1&fuzzy_nickname=al&role=1&incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListMemberInfosParams {
    /// Owner-user mode: list teams/memberships owned by this user. Mutually
    /// exclusive with `team_id`; when set, `role` and `fuzzy_nickname` must be
    /// omitted.
    pub owner_id: Option<String>,

    /// Team mode: list members of this team. Mutually exclusive with
    /// `owner_id`.
    pub team_id: Option<String>,

    /// Substring filter on member nickname (team mode only).
    pub fuzzy_nickname: Option<String>,

    /// Single role-bit filter (team mode only). Must be a singular valid role
    /// bit; composite values are rejected.
    pub role: Option<RoleField>,

    /// Related rows to embed. Repeatable. Values: `user`, `team`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<MemberInclOpt>,

    pub offset: u32,
    pub limit: u32,
}

impl TryInto<MemberListSpec> for ListMemberInfosParams {
    type Error = BaseError;

    fn try_into(self) -> BaseResult<MemberListSpec> {
        //
        let invalid_args_err = || BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-team-or-user-required"),
        };

        if self.owner_id.is_some() == self.team_id.is_some() {
            return Err(invalid_args_err());
        }

        if self.owner_id.is_some() && self.role.is_some() {
            return Err(invalid_args_err());
        }

        if let Some(owner_id) = self.owner_id {
            return accept(MemberListSpec::User {
                owner_id,
                incl_opt: self.incl_opt,
                offset: self.offset,
                limit: self.limit,
            });
        }

        accept(MemberListSpec::Team {
            team_id: self.team_id.ok_or_else(invalid_args_err)?,
            fuzzy_nickname: self.fuzzy_nickname,
            role: self.role,
            incl_opt: self.incl_opt,
            offset: self.offset,
            limit: self.limit,
        })
    }
}

/// Input parameters for updating a member's roles.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateMemberRolesParams {
    pub id: String,
    pub roles: RoleMask,
}
