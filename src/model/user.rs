use time::OffsetDateTime;

use crate::atom::auth;

pub struct UserToken {
    pub user_id: String,
}

pub struct UserInfo {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: i64,

    pub is_sadmin: bool,

    pub last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub struct UserForm {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub password_hash: String,
}

pub struct UserAvatarReservation {
    pub object_key: String,
    pub previous_object_key: Option<String>,
    pub avatar_version: i64,
}

pub struct UserCredential {
    pub user_id: String,
    pub password_hash: String,
}

impl UserCredential {
    pub fn verify_password(&self, password: &str) -> bool {
        auth::verify_password(password, &self.password_hash)
    }
}
