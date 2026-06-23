//! Step types for workset repository operations.

use poprako_transactional::step::Step;

use crate::model::workset::WorksetInfo;

/// Step that lists all worksets for a team with a pessimistic lock.
pub struct ListByTeamIdExcluded<'a> {
    pub team_id: &'a str,
}

impl<'a> Step for ListByTeamIdExcluded<'a> {
    type Output = Vec<WorksetInfo>;
}

/// Step that deletes a workset and all of its child data.
pub struct DeleteCascade<'a> {
    pub id: &'a str,
}

impl<'a> Step for DeleteCascade<'a> {
    type Output = ();
}

/// Factory for constructing workset repository [`Step`] values.
pub struct WorksetStep;

impl WorksetStep {
    /// Constructs a step to list a team's worksets with a pessimistic lock.
    pub fn list_by_team_id_excluded<'a>(team_id: &'a str) -> ListByTeamIdExcluded<'a> {
        ListByTeamIdExcluded { team_id }
    }

    /// Constructs a step to cascade-delete a workset.
    pub fn delete_cascade<'a>(id: &'a str) -> DeleteCascade<'a> {
        DeleteCascade { id }
    }
}
