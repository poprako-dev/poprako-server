//! Domain models for team profile storage.

use time::OffsetDateTime;

use crate::value::image::{ImageExt, ImageHash};

/// A teamrecord as stored in the database.
///
/// Carries raw [`OffsetDateTime`] timestamps; convert to [`TeamInfoVal`] for
/// presentation.
///
/// [`TeamInfoVal`]: crate::data::val::team::TeamInfoVal
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
