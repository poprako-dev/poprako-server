use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggr::user::UserAggr;
use crate::infra::query::schema;

// ── Queryable / Selectable ─────────────────────────────────────────────────

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct UserRow {
    pub f_id: String,
    pub f_nickname: String,
    pub f_qid: String,
    pub f_is_sadmin: bool,
    pub f_avatar_key: Option<String>,
    pub f_avatar_source: Option<String>,
    pub f_avatar_uploaded: bool,
    pub f_last_active_at: OffsetDateTime,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

#[derive(Insertable)]
#[diesel(table_name = schema::t_user)]
pub struct UserEntry<'a> {
    pub f_id: &'a str,
    pub f_nickname: &'a str,
    pub f_qid: &'a str,
    pub f_password_hash: &'a str,
    pub f_last_active_at: OffsetDateTime,
    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<UserRow> for UserAggr {
    fn from(v: UserRow) -> Self {
        UserAggr {
            id: v.f_id,
            nickname: v.f_nickname,
            qid: v.f_qid,
            is_sadmin: v.f_is_sadmin,
            avatar_key: v.f_avatar_key.unwrap_or_default(),
            avatar_uploaded: v.f_avatar_uploaded,
            last_active_at: v.f_last_active_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}
