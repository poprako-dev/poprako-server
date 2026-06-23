//! Domain models for worksets — units of work within a team.

/// A minimal workset record.
///
/// Worksets are scoped to a team and identified by an opaque id. Additional
/// metadata (name, description, status) is expected to be added as the
/// workset domain evolves.
#[cfg_attr(test, derive(Clone))]
pub struct WorksetInfo {
    pub id: String,
    pub team_id: String,
}
