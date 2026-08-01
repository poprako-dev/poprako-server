//! Diesel entity types for the `t_user` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::part_impl::repo::rdb_impl::schema::t_user;
use crate::result::{BaseError, BaseRest, accept};
use crate::value::image::{ImageExt, ImageHash};

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_user` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_user)]
pub struct UserInfoRow {
    //
    pub f_id: String,
    pub f_nickname: String,
    pub f_qid: String,

    pub f_is_sadmin: bool,

    pub f_avatar_key: Option<String>,
    pub f_avatar_uploaded: bool,
    #[diesel(deserialize_as = i64)]
    pub f_avatar_version: u32,
    pub f_avatar_hash: Vec<u8>,
    pub f_avatar_extension: String,

    pub f_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

/// Raw database row for user credentials (password hash) from the `t_user`
/// table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_user)]
pub struct UserCredsRow {
    //
    pub f_id: String,

    pub f_password_hash: String,
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_user` table.
#[derive(Insertable)]
#[diesel(table_name = t_user)]
pub struct UserEntryRow<'a> {
    //
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
pub struct UserAspectRow<'a> {
    //
    pub f_nickname: Option<&'a str>,
    pub f_qid: Option<&'a str>,

    pub f_avatar_key: Option<&'a str>,
    pub f_avatar_uploaded: Option<bool>,
    pub f_avatar_version: Option<i64>,
    pub f_avatar_hash: Option<&'a [u8]>,
    pub f_avatar_extension: Option<&'a str>,

    pub f_last_active_at: Option<OffsetDateTime>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> UserAspectRow<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_nickname: None,
            f_qid: None,
            f_avatar_key: None,
            f_avatar_uploaded: None,
            f_avatar_version: None,
            f_avatar_hash: None,
            f_avatar_extension: None,
            f_last_active_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn nickname(mut self, val: &'a str) -> Self {
        //
        self.f_nickname = Some(val);

        self
    }

    pub fn qid(mut self, val: &'a str) -> Self {
        //
        self.f_qid = Some(val);

        self
    }

    pub fn avatar_key(mut self, val: &'a str) -> Self {
        //
        self.f_avatar_key = Some(val);

        self
    }

    pub fn avatar_uploaded(mut self, val: bool) -> Self {
        //
        self.f_avatar_uploaded = Some(val);

        self
    }

    pub fn avatar_version(mut self, val: u32) -> Self {
        //
        self.f_avatar_version = Some(i64::from(val));

        self
    }

    pub fn avatar_hash(mut self, val: &'a ImageHash) -> Self {
        //
        self.f_avatar_hash = Some(val.as_bytes());

        self
    }

    pub fn avatar_ext(mut self, val: ImageExt) -> Self {
        //
        self.f_avatar_extension = Some(val.suffix());

        self
    }

    pub fn last_active_at(mut self, val: OffsetDateTime) -> Self {
        //
        self.f_last_active_at = Some(val);

        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl TryFrom<UserInfoRow> for UserInfo {
    type Error = BaseError;

    fn try_from(v: UserInfoRow) -> BaseRest<Self> {
        //
        let (avatar_hash_bytes, avatar_ext) = (
            v.f_avatar_hash.try_into().map_err(|_| {
                BaseError::Unrecoverable {
                    message:
                        "[UserInfoRow] f_avatar_hash must contain 32 bytes"
                            .into(),
                }
            })?,
            ImageExt::parse(&v.f_avatar_extension).ok_or_else(|| {
                BaseError::Unrecoverable {
                    message:
                        "[UserInfoRow] f_avatar_extension must be supported"
                            .into(),
                }
            })?,
        );

        accept(UserInfo {
            id: v.f_id,
            qid: v.f_qid,
            nickname: v.f_nickname,
            avatar_key: v.f_avatar_key,
            is_avatar_uploaded: v.f_avatar_uploaded,
            avatar_version: v.f_avatar_version,
            avatar_hash: ImageHash::new(avatar_hash_bytes),
            avatar_ext,
            is_sadmin: v.f_is_sadmin,
            last_active_at: v.f_last_active_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        })
    }
}

impl From<UserCredsRow> for UserCredential {
    fn from(v: UserCredsRow) -> Self {
        UserCredential {
            user_id: v.f_id,
            password_hash: v.f_password_hash,
        }
    }
}
