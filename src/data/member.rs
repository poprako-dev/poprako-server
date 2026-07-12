//! Data transfer objects for member use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::i18n::trl;
use poprako_util::time::ToUnixMilli;

use crate::data::{team_data, user_data};
use crate::model::member_model;
use crate::part::image::ImagePool;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

/// Presentation-ready membership information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct InfoVal {
    pub id: String,

    pub user_id: String,
    pub nickname: String,
    pub last_active_at: i64,

    pub team_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<user_data::InfoVal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<team_data::InfoVal>,

    pub roles: RoleMask,
}

impl From<member_model::Info> for InfoVal {
    fn from(value: member_model::Info) -> Self {
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

impl InfoVal {
    /// Converts a member model into a presentation-ready value,
    /// resolving included user/team data when present.
    pub async fn from_model<P>(
        image_pool: &P,
        model: member_model::Info,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let user = match model.user {
            //
            Some(user_info) => Some(
                user_data::InfoVal::from_model(image_pool, user_info).await?,
            ),

            None => None,
        };

        let team = match model.team {
            //
            Some(team_info) => Some(
                team_data::InfoVal::from_model(image_pool, team_info).await?,
            ),

            None => None,
        };

        Ok(Self {
            id: model.id,
            user_id: model.user_id,
            nickname: model.user_nickname,
            last_active_at: model.user_last_active_at.to_unix_milli(),
            team_id: model.team_id,
            user,
            team,
            roles: model.roles,
        })
    }
}

/// Input parameters for creating a member.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateData {
    pub user_id: String,
    pub team_id: String,

    pub roles: RoleMask,
}

/// Return value from creating a member.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateVal {
    pub id: String,
}

/// Input parameters for joining a team through a member invitation.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct JoinTeamData {
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
pub struct ListInfosData {
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

impl TryInto<member_model::ListSpec> for ListInfosData {
    type Error = RegularError;

    fn try_into(self) -> RegularResult<member_model::ListSpec> {
        //
        let invalid_args_err = || RegularError::Expected {
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
            return Ok(member_model::ListSpec::User {
                owner_id,
                incl_opt: self.incl_opt,
                offset: self.offset,
                limit: self.limit,
            });
        }

        Ok(member_model::ListSpec::Team {
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
pub struct UpdateRolesData {
    pub id: String,
    pub roles: RoleMask,
}
