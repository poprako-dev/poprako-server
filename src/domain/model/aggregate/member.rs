use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::model::aggregate::PrivateMarker;
use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::model::aggregate::user::UserAggr;
use crate::domain::model::value::role::RoleMask;

#[cfg_attr(test, derive(Clone))]
pub struct MemberAggr {
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
    pub assigned_assistant_at: Option<OffsetDateTime>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl MemberAggr {
    pub fn generate_id() -> String {
        format!("member-{}", Uuid::now_v7())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        user_id: String,
        user: Option<UserAggr>,
        team_id: String,
        team: Option<TeamAggr>,
        assigned_raw_provider_at: Option<OffsetDateTime>,
        assigned_translator_at: Option<OffsetDateTime>,
        assigned_proofreader_at: Option<OffsetDateTime>,
        assigned_typesetter_at: Option<OffsetDateTime>,
        assigned_redrawer_at: Option<OffsetDateTime>,
        assigned_reviewer_at: Option<OffsetDateTime>,
        assigned_publisher_at: Option<OffsetDateTime>,
        assigned_admin_at: Option<OffsetDateTime>,
        assigned_assistant_at: Option<OffsetDateTime>,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            user,
            team_id,
            team,
            assigned_raw_provider_at,
            assigned_translator_at,
            assigned_proofreader_at,
            assigned_typesetter_at,
            assigned_redrawer_at,
            assigned_reviewer_at,
            assigned_publisher_at,
            assigned_admin_at,
            assigned_assistant_at,
            created_at,
            updated_at,
            _m: PrivateMarker,
        }
    }
}



pub struct MemberForm {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub roles: RoleMask,

    /// Private marker to forbid struct literal construction outside this module.
    _m: PrivateMarker,
}

impl MemberForm {
    pub fn new(user_id: String, user_nickname: String, team_id: String, roles: RoleMask) -> Self {
        Self {
            id: MemberAggr::generate_id(),
            user_id,
            user_nickname,
            team_id,
            roles,
            _m: PrivateMarker,
        }
    }
}
