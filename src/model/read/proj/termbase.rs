//! Domain models for team- and comic-scoped terminology bases.

use time::OffsetDateTime;

/// A persisted terminology base.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseInfo {
    //
    /// The unique identifier for this terminology base.
    pub id: String,

    /// Foreign key to the team that owns this termbase, if team-scoped.
    pub team_id: Option<String>,
    /// Foreign key to the comic this termbase is associated with, if comic-scoped.
    pub comic_id: Option<String>,

    /// Display name of this terminology base.
    pub name: String,
    /// Free-text description of this terminology base's purpose or scope.
    pub description: Option<String>,

    /// Denormalised count of terms stored in this terminology base.
    pub term_count: usize,

    /// Foreign key to the user who created this termbase.
    pub creator_id: String,

    /// Timestamp when this terminology base was created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this terminology base was last modified.
    pub updated_at: OffsetDateTime,
}
