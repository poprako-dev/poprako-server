use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggr::team::TeamAggr;
use crate::infra::query::schema;

// ── Queryable / Selectable ─────────────────────────────────────────────────

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_team)]
pub struct TeamRow {
    pub f_id: String,
    pub f_name: String,
    pub f_description: Option<String>,
    pub f_avatar_key: Option<String>,
    pub f_avatar_uploaded: bool,
    pub f_workset_next_index: i32,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<TeamRow> for TeamAggr {
    fn from(v: TeamRow) -> Self {
        TeamAggr {
            id: v.f_id,
            name: v.f_name,
            description: v.f_description.unwrap_or_default(),
            avatar_key: v.f_avatar_key.unwrap_or_default(),
            avatar_uploaded: v.f_avatar_uploaded,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}
