//! Domain models for team profile storage.

use time::OffsetDateTime;

use crate::value::image::{ImageExt, ImageHash};

/// A teamrecord as stored in the database.
///
/// Carries raw [`OffsetDateTime`] timestamps; convert to [`TeamInfoVal`] for
/// presentation.
///
/// [`TeamInfoVal`]: crate::data::team::TeamInfoVal
#[derive(Clone)]
pub struct TeamInfo {
    //
    /// The unique identifier for this team.
    pub id: String,

    /// Display name of the team.
    pub name: String,
    /// Free-text description of the team's purpose or scope.
    pub description: String,

    /// Object-storage key for the team avatar image.
    pub avatar_key: Option<String>,
    /// Whether the team avatar has been uploaded and confirmed.
    pub is_avatar_uploaded: bool,
    /// Monotonically increasing version counter for the avatar.
    pub avatar_version: u32,
    /// SHA-256 identity of the reserved avatar content.
    pub avatar_hash: ImageHash,
    /// File format persisted with the avatar identity.
    pub avatar_ext: ImageExt,

    /// Timestamp when this team was created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this team was last modified.
    pub updated_at: OffsetDateTime,
}

/// Filtering and pagination parameters for listing teams.
pub struct TeamInfoListSpec {
    //
    /// Membership filter mode for the team listing.
    pub kind: TeamInfoListKind,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: u32,
}

/// Membership filtering mode for listing teams.
pub enum TeamInfoListKind {
    /// Include all teams.
    All,

    /// Include only teams joined by the specified user.
    JoinedBy {
        /// ID of the user whose team memberships to list.
        user_id: String,
    },
}

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

/// The result of reserving a new team avatar upload slot.
///
/// Mirrors [`UserAvatarReservation`] for the team domain. Contains the
/// generated object key, any previous key to clean up, and the version
/// that must match when the upload is confirmed.
///
/// [`UserAvatarReservation`]: crate::model::user::UserAvatarReservation
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
