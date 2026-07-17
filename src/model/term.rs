//! Domain models for terms stored inside terminology bases.

use time::OffsetDateTime;

/// A persisted terminology entry.
#[cfg_attr(test, derive(Clone))]
pub struct TermInfo {
    pub id: String,

    pub termbase_id: String,

    pub source: String,
    pub targets: Vec<String>,
    pub comment: Option<String>,

    pub creator_id: String,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to create a terminology entry.
#[cfg_attr(test, derive(Clone))]
pub struct TermEntry {
    pub id: String,

    pub termbase_id: String,

    pub source: String,
    pub targets: Vec<String>,
    pub comment: Option<String>,

    pub creator_id: String,
}

/// Mutable terminology-entry fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermInfoUpdate {
    pub id: String,

    pub source: String,
    pub targets: Vec<String>,
    pub comment: Option<String>,
}

/// Filtering and pagination parameters for terms inside one terminology base.
pub struct TermInfoListSpec {
    pub termbase_id: String,

    pub fuzzy_source: Option<String>,

    pub offset: u32,
    pub limit: u32,
}
