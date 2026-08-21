//! Domain models for user authentication and profile storage.

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

/// Mutable user profile fields replaced together.
pub struct UserInfoRepl {
    //
    /// The user identifier.
    pub id: String,

    /// The OAuth qualified identifier.
    pub qid: String,
    /// The display nickname.
    pub nickname: String,
}

/// A user's credential fields replaced together.
pub struct UserCredsRepl {
    //
    /// The user identifier.
    pub id: String,

    /// The new password hash.
    pub password_hash: String,
}

/// A user's avatar upload state replaced together.
pub struct UserAvatarRepl {
    //
    /// The user identifier.
    pub id: String,

    /// The avatar version being confirmed.
    pub avatar_version: u32,
    /// The object-storage key, when the upload has an object identity.
    pub avatar_key: Option<String>,
    /// Whether the upload has completed.
    pub is_avatar_uploaded: bool,
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
    pub is_upload_required: bool,
}
