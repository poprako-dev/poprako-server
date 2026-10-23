//! Diesel entity types for the `t_user` table.

use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::part_impl::repo::rdb_impl::schema::t_user;
use crate::result::{BaseError, BaseRest, accept};

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database record for the `t_user` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_user)]
pub struct UserInfoRecord {
    pub f_id: String,

    pub f_nickname: String,
    pub f_qid: String,

    pub f_is_sadmin: bool,

    pub f_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

/// Raw database record for user credentials (password hash) from the `t_user`
/// table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_user)]
pub struct UserCredsRecord {
    pub f_id: String,

    pub f_password_hash: String,
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_user` table.
#[derive(Insertable)]
#[diesel(table_name = t_user)]
pub struct UserItem<'a> {
    pub f_id: &'a str,

    pub f_nickname: &'a str,
    pub f_qid: &'a str,

    pub f_password_hash: &'a str,

    pub f_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Aspect struct for updating specific fields of a user record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_user)]
pub struct UserPack<'a> {
    pub f_nickname: Option<&'a str>,
    pub f_qid: Option<&'a str>,

    pub f_last_active_at: Option<OffsetDateTime>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> UserPack<'a> {
    #[must_use]
    pub const fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_nickname: None,
            f_qid: None,
            f_last_active_at: None,
            f_updated_at: updated_at,
        }
    }

    #[must_use]
    pub const fn nickname(mut self, val: &'a str) -> Self {
        //
        self.f_nickname = Some(val);

        self
    }

    #[must_use]
    pub const fn qid(mut self, val: &'a str) -> Self {
        //
        self.f_qid = Some(val);

        self
    }

    #[must_use]
    pub const fn last_active_at(mut self, val: OffsetDateTime) -> Self {
        //
        self.f_last_active_at = Some(val);

        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl TryFrom<UserInfoRecord> for UserInfo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    fn try_from(record: UserInfoRecord) -> BaseRest<Self> {
        //
        accept(Self {
            id: record.f_id,
            qid: record.f_qid,
            nickname: record.f_nickname,
            is_sadmin: record.f_is_sadmin,
            last_active_at: record.f_last_active_at,
            created_at: record.f_created_at,
            updated_at: record.f_updated_at,
        })
    }
}

impl From<UserCredsRecord> for UserCredential {
    fn from(record: UserCredsRecord) -> Self {
        Self {
            user_id: record.f_id,
            password_hash: record.f_password_hash,
        }
    }
}
