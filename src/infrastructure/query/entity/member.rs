use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member::MemberAggr;
use crate::infrastructure::query::schema;

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
pub struct MemberEntry<'a> {
    pub f_id: &'a str,
    pub f_user_id: &'a str,
    pub f_user_nickname: &'a str,
    pub f_team_id: &'a str,
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

impl From<MemberRow> for MemberAggr {
    fn from(v: MemberRow) -> Self {
        MemberAggr::new(
            v.f_id,
            v.f_user_id,
            None,
            v.f_team_id,
            None,
            v.f_assigned_raw_provider_at,
            v.f_assigned_translator_at,
            v.f_assigned_proofreader_at,
            v.f_assigned_typesetter_at,
            v.f_assigned_redrawer_at,
            v.f_assigned_reviewer_at,
            v.f_assigned_publisher_at,
            v.f_assigned_admin_at,
            v.f_created_at,
            v.f_updated_at,
        )
    }
}
