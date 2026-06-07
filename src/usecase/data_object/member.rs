use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::member::MemberAggr;
use crate::usecase::data_object::team::TeamBase;
use crate::usecase::data_object::user::UserBase;

/// Public-facing representation of a team member.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberBase {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,
    pub user: Option<UserBase>,

    pub team_id: String,
    pub team: Option<TeamBase>,

    pub roles: u32,

    pub user_last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl MemberBase {
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
            Some(UserBase::from_aggr(u, signer).await)
        } else {
            None
        };

        let team = if let Some(t) = team_aggr {
            Some(TeamBase::from_aggr(t, signer).await)
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemberRoleUpdateParams {
    pub roles: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemberCreateParams {
    pub user_id: String,
    pub team_id: String,
    pub role_mask: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberCreateReply {
    pub id: String,
}
