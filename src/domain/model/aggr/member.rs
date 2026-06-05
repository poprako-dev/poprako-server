use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::model::aggr::team::TeamAggr;
use crate::domain::model::aggr::user::UserAggr;
use crate::domain::model::value::role::RoleMask;

#[cfg_attr(test, derive(Clone))]
pub struct MemberAggr {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,
    pub user: Option<UserAggr>,

    pub team_id: String,
    pub team: Option<TeamAggr>,

    pub assigned_raw_provider_at: Option<OffsetDateTime>,
    pub assigned_translator_at: Option<OffsetDateTime>,
    pub assigned_proofreader_at: Option<OffsetDateTime>,
    pub assigned_typesetter_at: Option<OffsetDateTime>,
    pub assigned_redrawer_at: Option<OffsetDateTime>,
    pub assigned_reviewer_at: Option<OffsetDateTime>,
    pub assigned_publisher_at: Option<OffsetDateTime>,
    pub assigned_admin_at: Option<OffsetDateTime>,
    pub assigned_assistant_at: Option<OffsetDateTime>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MemberAggr {
    pub fn generate_id() -> String {
        format!("member-{}", Uuid::now_v7())
    }
}

pub struct MemberForm {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub roles: RoleMask,
}
