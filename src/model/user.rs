//! Domain models for user authentication and profile storage.

use time::OffsetDateTime;

/// A deserialized authentication token identifying a user session.
#[derive(Clone, Debug)]
pub struct UserToken {
    pub user_id: String,
}

/// A borrowed reference to a user authentication token, used in middleware
/// to avoid cloning the owned [UserToken].
pub struct UserTokenRef<'a> {
    pub user_id: &'a str,
}

/// A userprofile record as stored in the database.
///
/// Carries raw [`OffsetDateTime`] timestamps; convert to [`UserInfoVal`] for
/// presentation. Avatar fields track a multi-step upload flow: a key is
/// reserved, the client uploads to that key, then the upload is marked complete.
///
/// [`UserInfoVal`]: crate::data::user::UserInfoVal
#[derive(Clone)]
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

/// The data needed to insert a new user row.
#[cfg_attr(test, derive(Clone))]
pub struct UserForm {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub password_hash: String,
}

/// The result of reserving a new avatar upload slot.
///
/// Contains the generated object-storage key for the client to PUT to,
/// the previous key (if any) to clean up after the new upload succeeds,
/// and the version number that must match when marking the upload complete.
#[cfg_attr(test, derive(Clone))]
pub struct UserAvatarReservation {
    pub object_key: String,
    pub prev_object_key: Option<String>,
    pub avatar_version: i64,
}

/// A stored password credential used during login verification.
#[cfg_attr(test, derive(Clone))]
pub struct UserCredential {
    pub user_id: String,
    pub password_hash: String,
}
