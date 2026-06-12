use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli as _;

use poprako_util::i18n::trl;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::member::MemberAggr;
use crate::domain::model::value::role::RoleFlag;
use crate::domain::result::{DomainError, DomainResult};
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
pub struct RoleUpdateParams {
    pub roles: u32,
}

/// Request body for creating a member directly (sadmin only).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateParams {
    pub user_id: String,
    pub team_id: String,
    pub role_mask: u32,
}

/// Response body after creating a member.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateReply {
    pub id: String,
}

/// Request body for joining a team by invitation code.
#[derive(Debug, Deserialize, ToSchema)]
pub struct JoinParams {
    pub invitation_code: String,
}

/// Query parameters for listing members.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct ListParams {
    pub team_id: Option<String>,
    pub user_id: Option<String>,
    pub keyword: Option<String>,
    pub role: Option<u32>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub includes: Vec<String>,
}

impl ListParams {
    /// Validates query parameters for member listing.
    ///
    /// Checks that `offset` is non-negative, `limit` is in [1, 200],
    /// and `role` (if present) is a single bit value.
    pub fn validate(&self) -> DomainResult<()> {
        if let Some(offset) = self.offset
            && offset < 0
        {
            return Err(DomainError::expected_argument(trl("error-invalid-offset")));
        }

        if let Some(limit) = self.limit
            && !(1..31).contains(&limit)
        {
            return Err(DomainError::expected_argument(trl("error-invalid-limit")));
        }

        if let Some(role) = self.role
            && RoleFlag::try_from_single_bit(role).is_none()
        {
            return Err(DomainError::expected_argument(trl("error-invalid-role")));
        }

        if self.team_id.is_none() && self.user_id.is_none() {
            // FIXME: ftl
            return Err(DomainError::expected_argument(trl(
                "error-team-or-user-required",
            )));
        }

        Ok(())
    }
}
