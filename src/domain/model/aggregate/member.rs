use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::model::aggregate::user::UserAggr;
use crate::domain::model::value::role::RoleMask;

pub struct Member {
    pub id: String,

    pub user_id: String,
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

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Member {
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

impl MemberForm {
    pub fn new(user_id: String, user_nickname: String, team_id: String, roles: RoleMask) -> Self {
        Self {
            id: Member::generate_id(),
            user_id,
            user_nickname,
            team_id,
            roles,
        }
    }
}
