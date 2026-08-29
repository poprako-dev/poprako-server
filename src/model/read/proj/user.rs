//! Domain models for user authentication and profile storage.

use time::OffsetDateTime;

/// A userprofile record as stored in the database.
///
/// Carries raw [`OffsetDateTime`] timestamps; convert to [`UserInfoView`] for
/// presentation. Avatar fields track a multi-step upload flow: a key is
/// reserved, the client uploads to that key, then the upload is marked complete.
///
/// [`UserInfoView`]: crate::data::view::user::UserInfoView
#[derive(Clone)]
pub struct UserInfo {
    /// Server-assigned unique user identifier.
    pub id: String,

    /// Third-party OAuth or QID provider identifier for this user.
    pub qid: String,
    /// Display name shown throughout the application.
    pub nickname: String,

    /// Whether this user has super-administrator privileges.
    pub is_sadmin: bool,

    /// Timestamp of the user's most recent activity.
    pub last_active_at: OffsetDateTime,

    /// Timestamp when this user was created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this user was last modified.
    pub updated_at: OffsetDateTime,
}

/// A stored password credential used during login verification.
#[cfg_attr(test, derive(Clone))]
pub struct UserCredential {
    /// Foreign key referencing the user this credential belongs to.
    pub user_id: String,
    /// Bcrypt or similar hashed password for login verification.
    pub password_hash: String,
}
