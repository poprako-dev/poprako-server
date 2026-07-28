//! Domain models for user authentication and profile storage.

use time::OffsetDateTime;

use crate::value::image::{ImageExt, ImageHash};

/// A userprofile record as stored in the database.
///
/// Carries raw [`OffsetDateTime`] timestamps; convert to [`UserInfoVal`] for
/// presentation. Avatar fields track a multi-step upload flow: a key is
/// reserved, the client uploads to that key, then the upload is marked complete.
///
/// [`UserInfoVal`]: crate::data::val::user::UserInfoVal
#[derive(Clone)]
pub struct UserInfo {
    //
    /// Server-assigned unique user identifier.
    pub id: String,

    /// Third-party OAuth or QID provider identifier for this user.
    pub qid: String,
    /// Display name shown throughout the application.
    pub nickname: String,

    /// Object-storage key for the uploaded avatar image, absent when no avatar is set.
    pub avatar_key: Option<String>,
    /// Whether the reserved avatar upload has been completed.
    pub is_avatar_uploaded: bool,
    /// Monotonically increasing version number for cache-busting the avatar URL.
    pub avatar_version: u32,
    /// SHA-256 identity of the reserved avatar content.
    pub avatar_hash: ImageHash,
    /// File format persisted with the avatar identity.
    pub avatar_ext: ImageExt,

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
    //
    /// Foreign key referencing the user this credential belongs to.
    pub user_id: String,
    /// Bcrypt or similar hashed password for login verification.
    pub password_hash: String,
}
