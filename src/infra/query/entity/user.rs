use diesel::prelude::*;
use time::OffsetDateTime;

use crate::infra::query::schema;

// UserInfo is the result of a selection.
#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::tbl_user)]
pub struct UserInfo {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    pub is_sadmin: bool,

    pub avatar_key: Option<String>,
    pub avatar_source: Option<String>,
    pub avatar_uploaded: bool,

    pub last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// UserCredential is the result of a selection, but only contains fields related to authentication.
#[derive(Queryable)]
#[diesel(table_name = schema::tbl_user)]
pub struct UserCredential {
    pub qid: String,
    pub password_hash: String,
}

// UserEntry is the struct used for only inserting a user.
#[derive(Insertable)]
#[diesel(table_name = schema::tbl_user)]
pub struct UserEntry {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    // password_hash must be provided when inserting a new user.
    pub password_hash: String,

    pub last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// NOTE: No update struct for now, as update logics are complecated and
// are not so easy to be contained in one single struct.
