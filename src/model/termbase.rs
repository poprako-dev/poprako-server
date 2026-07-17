//! Domain models for team- and comic-scoped terminology bases.

use time::OffsetDateTime;

/// A persisted terminology base.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseInfo {
    pub id: String,

    pub team_id: Option<String>,
    pub comic_id: Option<String>,

    pub name: String,
    pub description: Option<String>,

    pub term_count: i32,

    pub creator_id: String,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to create a terminology base.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseEntry {
    pub id: String,

    pub team_id: Option<String>,
    pub comic_id: Option<String>,

    pub name: String,
    pub description: Option<String>,

    pub creator_id: String,
}

/// Mutable terminology-base profile fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseInfoUpdate {
    pub id: String,

    pub name: String,
    pub description: Option<String>,
}

/// Filtering and pagination parameters for terminology-base lists.
pub enum TermbaseInfoListSpec {
    /// List terminology bases directly owned by a team.
    Team {
        team_id: String,
        fuzzy_name: Option<String>,
        offset: u32,
        limit: u32,
    },
    /// List terminology bases visible from a comic.
    Comic {
        team_id: String,
        comic_id: String,
        fuzzy_name: Option<String>,
        offset: u32,
        limit: u32,
    },
}
