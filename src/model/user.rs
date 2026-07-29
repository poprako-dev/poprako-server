//! Domain models for user authentication and profile storage.

use time::OffsetDateTime;

use crate::value::image::{ImageExt, ImageHash};

/// A deserialized authentication token identifying a user session.
#[derive(Clone, Debug)]
pub struct UserToken {
    /// Identifier of the user this token authenticates.
    pub user_id: String,
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
    pub avatar_uploaded: bool,
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

/// The data needed to insert a new user row.
#[cfg_attr(test, derive(Clone))]
pub struct UserEntry {
    //
    /// Server-assigned unique user identifier.
    pub id: String,

    /// Third-party OAuth or QID provider identifier for this user.
    pub qid: String,
    /// Display name shown throughout the application.
    pub nickname: String,

    /// Bcrypt or similar hashed password for credential verification.
    pub password_hash: String,
}

/// The result of reserving a new avatar upload slot.
///
/// Contains the generated object-storage key for the client to PUT to,
/// the previous key (if any) to clean up after the new upload succeeds,
/// and the version number that must match when marking the upload complete.
#[cfg_attr(test, derive(Clone))]
pub struct UserAvatarReservation {
    //
    /// Generated object-storage key for the client to upload the new avatar to.
    pub object_key: String,
    /// Previous avatar key to delete after the new upload succeeds, absent when there was no prior avatar.
    pub prev_object_key: Option<String>,
    /// Expected version number that must match when confirming the upload.
    pub avatar_version: u32,
    /// Whether a PUT capability and delayed check are required.
    pub upload_required: bool,
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
