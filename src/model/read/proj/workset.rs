//! Domain models for worksets — units of work within a team.

use time::OffsetDateTime;

/// A minimal worksetrecord.
///
/// Worksets are scoped to a team and carry denormalized comic counters.
#[derive(Clone)]
pub struct WorksetInfo {
    /// Server-assigned unique workset identifier.
    pub id: String,

    /// Foreign key referencing the owning team.
    pub team_id: String,
    /// Display ordering index within the team.
    pub index: i32,

    /// Human-readable workset name.
    pub name: String,
    /// Optional longer description of this workset's purpose.
    pub description: Option<String>,

    /// Denormalized count of comics in this workset.
    pub comic_count: i32,

    /// Timestamp when this workset was created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this workset was last modified.
    pub updated_at: OffsetDateTime,
}
