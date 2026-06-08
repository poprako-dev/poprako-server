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
    pub f_avatar_version: i64,
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

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Changeset for updating user fields via partial updates.
///
/// Only `Some` fields are included in the generated `SET` clause;
/// `None` fields are omitted.
#[derive(AsChangeset)]
#[diesel(table_name = schema::t_user)]
pub struct UserAspect<'a> {
    pub f_nickname: Option<&'a str>,
    pub f_qid: Option<&'a str>,
    pub f_avatar_key: Option<&'a str>,
    pub f_avatar_uploaded: Option<bool>,
    pub f_avatar_version: Option<i64>,
    pub f_last_active_at: Option<OffsetDateTime>,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> UserAspect<'a> {
    /// Creates a new changeset with all optional fields set to `None`.
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_nickname: None,
            f_qid: None,
            f_avatar_key: None,
            f_avatar_uploaded: None,
            f_avatar_version: None,
            f_last_active_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn nickname(mut self, val: &'a str) -> Self {
        self.f_nickname = Some(val);
        self
    }

    pub fn qid(mut self, val: &'a str) -> Self {
        self.f_qid = Some(val);
        self
    }

    pub fn avatar_key(mut self, val: &'a str) -> Self {
        self.f_avatar_key = Some(val);
        self
    }

    pub fn avatar_uploaded(mut self, val: bool) -> Self {
        self.f_avatar_uploaded = Some(val);
        self
    }

    pub fn avatar_version(mut self, val: i64) -> Self {
        self.f_avatar_version = Some(val);
        self
    }

    pub fn last_active_at(mut self, val: OffsetDateTime) -> Self {
        self.f_last_active_at = Some(val);
        self
    }
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
            avatar_version: v.f_avatar_version,
            last_active_at: v.f_last_active_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}
