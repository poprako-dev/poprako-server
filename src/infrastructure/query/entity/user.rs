use diesel::prelude::*;
use time::OffsetDateTime;

use crate::domain::model::aggregate::user::UserAggr;
use crate::infrastructure::query::schema;

// ── Queryable / Selectable ─────────────────────────────────────────────────

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::t_user)]
pub struct UserInfo {
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

#[derive(Queryable)]
#[diesel(table_name = schema::t_user)]
pub struct UserCredentialRow {
    pub f_qid: String,
    pub f_password_hash: String,
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

impl From<UserInfo> for UserAggr {
    fn from(v: UserInfo) -> Self {
        UserAggr::new(
            v.f_id,
            v.f_nickname,
            v.f_qid,
            v.f_is_sadmin,
            v.f_avatar_key.unwrap_or_default(),
            v.f_avatar_uploaded,
            v.f_last_active_at,
            v.f_created_at,
            v.f_updated_at,
        )
    }
}
