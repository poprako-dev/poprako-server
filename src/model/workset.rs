//! Domain models for worksets — units of work within a team.

use time::OffsetDateTime;

/// A minimal worksetrecord.
///
/// Worksets are scoped to a team and keep denormalized comic counters for
/// list views and comic index allocation.
#[derive(Clone)]
pub struct WorksetInfo {
    pub id: String,

    pub team_id: String,
    pub index: i32,

    pub name: String,
    pub description: Option<String>,

    pub comic_count: i32,
    pub comic_next_index: i32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to create a new workset.
#[cfg_attr(test, derive(Clone))]
pub struct WorksetForm {
    pub id: String,

    pub team_id: String,
    pub index: i32,

    pub name: String,
    pub description: Option<String>,
}

/// Mutable profile fields for a workset.
#[cfg_attr(test, derive(Clone))]
pub struct WorksetInfoUpdate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}
