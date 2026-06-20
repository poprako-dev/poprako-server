use time::OffsetDateTime;

use crate::atom::auth;

#[cfg_attr(test, derive(Clone))]
pub struct UserToken {
    pub user_id: String,
}

#[cfg_attr(test, derive(Clone))]
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

#[cfg_attr(test, derive(Clone))]
pub struct UserForm {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub password_hash: String,
}

#[cfg_attr(test, derive(Clone))]
pub struct UserAvatarReservation {
    pub object_key: String,
    pub previous_object_key: Option<String>,
    pub avatar_version: i64,
}

#[cfg_attr(test, derive(Clone))]
pub struct UserCredential {
    pub user_id: String,
    pub password_hash: String,
}

impl UserCredential {
    pub fn verify_password(&self, password: &str) -> bool {
        auth::verify_password(password, &self.password_hash)
    }
}
