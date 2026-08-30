//! Domain models for team profile storage.

use time::OffsetDateTime;

/// A teamrecord as stored in the database.
///
/// Carries raw [`OffsetDateTime`] timestamps; convert to [`TeamInfoView`] for
/// presentation.
///
/// [`TeamInfoView`]: crate::data::view::team::TeamInfoView
#[derive(Clone)]
pub struct TeamInfo {
    //
    /// The unique identifier for this team.
    pub id: String,

    /// Display name of the team.
    pub name: String,
    /// Free-text description of the team's purpose or scope.
    pub description: String,

    /// Timestamp when this team was created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this team was last modified.
    pub updated_at: OffsetDateTime,
}
