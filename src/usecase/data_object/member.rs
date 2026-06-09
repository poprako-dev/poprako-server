use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use poprako_util::page::Page;
use poprako_util::time::ToUnixMilli as _;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::member::MemberAggr;
use crate::domain::model::value::member_inclusion::MemberInclusion;
use crate::domain::model::value::role::RoleFlag;
use crate::usecase::data_object::team::TeamInfo;
use crate::usecase::data_object::user::UserInfo;

/// Public-facing representation of a team member.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberInfo {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,
    pub user: Option<UserInfo>,

    pub team_id: String,
    pub team: Option<TeamInfo>,

    pub roles: u32,

    pub user_last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl MemberInfo {
    pub async fn from_aggr<S>(aggr: MemberAggr, signer: &S) -> Self
    where
        S: ImageGet,
    {
        // Compute roles before moving fields out of the aggregate.
        let roles: u32 = aggr.role_mask().into();

        // Extract user and team before consuming the remaining aggregate fields.
        let user_aggr = aggr.user;
        let team_aggr = aggr.team;

        let user = if let Some(u) = user_aggr {
            Some(UserInfo::from_aggr(u, signer).await)
        } else {
            None
        };

        let team = if let Some(t) = team_aggr {
            Some(TeamInfo::from_aggr(t, signer).await)
        } else {
            None
        };

        Self {
            id: aggr.id,
            user_id: aggr.user_id,
            user_nickname: aggr.user_nickname,
            user,
            team_id: aggr.team_id,
            team,
            roles,
            user_last_active_at: aggr.user_last_active_at.to_unix_milli(),
            created_at: aggr.created_at.to_unix_milli(),
            updated_at: aggr.updated_at.to_unix_milli(),
        }
    }
}

/// Request body for updating a member's roles.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MemberRoleUpdateParams {
    pub roles: u32,
}

/// Request body for creating a member directly (sadmin only).
#[derive(Debug, Deserialize, ToSchema)]
pub struct MemberCreateParams {
    pub user_id: String,
    pub team_id: String,
    pub role_mask: u32,
}

/// Response body after creating a member.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberCreateReply {
    pub id: String,
}

/// Request body for joining a team by invitation code.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MemberJoinParams {
    pub invitation_code: String,
}

/// Query parameters for listing members.
#[derive(Debug)]
pub struct MemberListParams {
    pub team_id: Option<String>,
    pub user_id: Option<String>,
    pub keyword: Option<String>,
    pub role: Option<RoleFlag>,
    pub page: Page,
    pub includes: MemberInclusion,
}

/// HTTP query parameters for listing members.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberListQuery {
    pub team_id: String,
    pub keyword: Option<String>,
    pub role: Option<u32>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub includes: Option<String>,
}

/// HTTP query parameters for listing the current user's memberships.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberMineQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub includes: Option<String>,
}
