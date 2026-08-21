//! Domain models for team profile storage.

/// The data needed to create a new team.
#[cfg_attr(test, derive(Clone))]
pub struct TeamEntry {
    //
    /// The unique identifier for the new team.
    pub id: String,

    /// Display name for the team being created.
    pub name: String,
    /// Free-text description for the team being created.
    pub description: String,
}

/// Mutable team profile fields replaced together.
pub struct TeamRepl {
    //
    /// The team identifier.
    pub id: String,

    /// The display name.
    pub name: String,
    /// The team description.
    pub description: String,
}

/// A team's avatar upload state replaced together.
pub struct TeamAvatarRepl {
    //
    /// The team identifier.
    pub id: String,

    /// The avatar version being confirmed.
    pub avatar_version: u32,
    /// The object-storage key, when the upload has an object identity.
    pub avatar_key: Option<String>,
    /// Whether the upload has completed.
    pub is_avatar_uploaded: bool,
}

/// The result of reserving a new team avatar upload slot.
///
/// Mirrors [`UserAvatarReservation`] for the team domain. Contains the
/// generated object key, any previous key to clean up, and the version
/// that must match when the upload is confirmed.
///
/// [`UserAvatarReservation`]: crate::model::write::user::UserAvatarReservation
#[cfg_attr(test, derive(Clone))]
pub struct TeamAvatarReservation {
    //
    /// Newly generated object-storage key for the avatar upload slot.
    pub object_key: String,
    /// Previous avatar key that should be cleaned up from storage, if any.
    pub prev_object_key: Option<String>,
    /// The new version number that must match on upload confirmation.
    pub avatar_version: u32,
    /// Whether a PUT capability and delayed check are required.
    /// TODO: what is this
    pub is_upload_required: bool,
}
