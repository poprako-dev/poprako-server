use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggr::member::Member;
use crate::infra::query::schema;

// ── Queryable / Selectable ─────────────────────────────────────────────────

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_member)]
pub struct MemberRow {
    pub f_id: String,
    pub f_user_id: String,
    pub f_user_nickname: String,
    pub f_team_id: String,
    pub f_assigned_raw_provider_at: Option<OffsetDateTime>,
    pub f_assigned_translator_at: Option<OffsetDateTime>,
    pub f_assigned_proofreader_at: Option<OffsetDateTime>,
    pub f_assigned_typesetter_at: Option<OffsetDateTime>,
    pub f_assigned_redrawer_at: Option<OffsetDateTime>,
    pub f_assigned_reviewer_at: Option<OffsetDateTime>,
    pub f_assigned_publisher_at: Option<OffsetDateTime>,
    pub f_assigned_admin_at: Option<OffsetDateTime>,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

#[derive(Insertable)]
#[diesel(table_name = schema::t_member)]
pub struct MemberEntry {
    pub f_id: String,
    pub f_user_id: String,
    pub f_user_nickname: String,
    pub f_team_id: String,
    pub f_assigned_raw_provider_at: Option<OffsetDateTime>,
    pub f_assigned_translator_at: Option<OffsetDateTime>,
    pub f_assigned_proofreader_at: Option<OffsetDateTime>,
    pub f_assigned_typesetter_at: Option<OffsetDateTime>,
    pub f_assigned_redrawer_at: Option<OffsetDateTime>,
    pub f_assigned_reviewer_at: Option<OffsetDateTime>,
    pub f_assigned_publisher_at: Option<OffsetDateTime>,
    pub f_assigned_admin_at: Option<OffsetDateTime>,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<MemberRow> for Member {
    fn from(v: MemberRow) -> Self {
        Self {
            id: v.f_id,
            user_id: v.f_user_id,
            user: None,
            team_id: v.f_team_id,
            team: None,
            assigned_raw_provider_at: v.f_assigned_raw_provider_at,
            assigned_translator_at: v.f_assigned_translator_at,
            assigned_proofreader_at: v.f_assigned_proofreader_at,
            assigned_typesetter_at: v.f_assigned_typesetter_at,
            assigned_redrawer_at: v.f_assigned_redrawer_at,
            assigned_reviewer_at: v.f_assigned_reviewer_at,
            assigned_publisher_at: v.f_assigned_publisher_at,
            assigned_admin_at: v.f_assigned_admin_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}
