use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::model::aggr::team::TeamAggr;

/// Read-model aggregate for a workset (collection of comics owned by a team).
#[cfg_attr(test, derive(Clone))]
pub struct WorksetAggr {
    pub id: String,

    pub team_id: String,
    pub team: Option<TeamAggr>,

    pub index: i32,
    pub name: String,
    pub description: Option<String>,
    pub comic_count: i32,
    pub comic_next_index: i32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl WorksetAggr {
    pub fn generate_id() -> String {
        format!("workset-{}", Uuid::now_v7())
    }
}

/// Input aggregate for creating a new workset.
///
/// The caller must generate `id` via [`WorksetAggr::generate_id`] and provide
/// a team-scoped `index` (typically from `TeamAggr::workset_next_index`).
pub struct WorksetForm {
    pub id: String,

    pub team_id: String,
    pub index: i32,

    pub name: String,
    pub description: Option<String>,
}

/// Input aggregate for updating a workset (PUT semantics).
///
/// The caller provides the existing workset `id`.
pub struct WorksetUpdate {
    pub id: String,

    pub name: String,
    pub description: Option<String>,
}
