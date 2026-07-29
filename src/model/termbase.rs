//! Domain models for team- and comic-scoped terminology bases.

use time::OffsetDateTime;

/// A persisted terminology base.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseInfo {
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
    pub term_count: i32,

    /// Foreign key to the user who created this termbase.
    pub creator_id: String,

    /// Timestamp when this terminology base was created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this terminology base was last modified.
    pub updated_at: OffsetDateTime,
}

/// The data needed to create a terminology base.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseEntry {
    /// The unique identifier for the new terminology base.
    pub id: String,

    /// Foreign key of the owning team, if creating a team-scoped termbase.
    pub team_id: Option<String>,
    /// Foreign key of the associated comic, if creating a comic-scoped termbase.
    pub comic_id: Option<String>,

    /// Display name for the new terminology base.
    pub name: String,
    /// Free-text description for the new terminology base.
    pub description: Option<String>,

    /// Foreign key of the user creating this terminology base.
    pub creator_id: String,
}

/// Mutable terminology-base profile fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseInfoUpdate {
    /// The unique identifier of the termbase to update.
    pub id: String,

    /// Updated display name for the terminology base.
    pub name: String,
    /// Updated description for the terminology base.
    pub description: Option<String>,
}

/// Filtering and pagination parameters for terminology-base lists.
pub enum TermbaseInfoListSpec {
    /// List terminology bases directly owned by a team.
    Team {
        /// ID of the team whose termbases to list.
        team_id: String,
        /// Optional fuzzy name filter.
        fuzzy_name: Option<String>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: u32,
    },
    /// List terminology bases visible from a comic.
    Comic {
        /// ID of the team that owns the comic.
        team_id: String,
        /// ID of the comic whose associated termbases to list.
        comic_id: String,
        /// Optional fuzzy name filter.
        fuzzy_name: Option<String>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: u32,
    },
}
