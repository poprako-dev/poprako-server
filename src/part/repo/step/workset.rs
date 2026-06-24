//! Step types for workset repository operations.

use poprako_transactional::step::Step;

use crate::model::workset::{WorksetForm, WorksetInfo, WorksetInfoUpdate};

/// Step that inserts a new workset row.
pub struct Create<'a> {
    pub form: &'a WorksetForm,
}

impl<'a> Step for Create<'a> {
    type Output = WorksetInfo;
}

/// Step that fetches a workset by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = WorksetInfo;
}

/// Step that lists all worksets for a team.
pub struct ListByTeamId<'a> {
    pub team_id: &'a str,
}

impl<'a> Step for ListByTeamId<'a> {
    type Output = Vec<WorksetInfo>;
}

/// Step that updates a workset's profile fields.
pub struct UpdateInfo<'a> {
    pub update: &'a WorksetInfoUpdate,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

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

/// Step that allocates one comic index from a workset-scoped sequence.
pub struct IncrementComicNextIndex<'a> {
    pub id: &'a str,
}

impl<'a> Step for IncrementComicNextIndex<'a> {
    type Output = i32;
}

/// Step that applies a delta to a workset's comic counter.
pub struct UpdateComicCount<'a> {
    pub id: &'a str,
    pub delta: i32,
}

impl<'a> Step for UpdateComicCount<'a> {
    type Output = ();
}

/// Factory for constructing workset repository [`Step`] values.
pub struct WorksetStep;

impl WorksetStep {
    /// Constructs a step to insert a new workset.
    pub fn create<'a>(form: &'a WorksetForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to fetch a workset by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to list a team's worksets.
    pub fn list_by_team_id<'a>(team_id: &'a str) -> ListByTeamId<'a> {
        ListByTeamId { team_id }
    }

    /// Constructs a step to update a workset's profile fields.
    pub fn update_info<'a>(update: &'a WorksetInfoUpdate) -> UpdateInfo<'a> {
        UpdateInfo { update }
    }

    /// Constructs a step to list a team's worksets with a pessimistic lock.
    pub fn list_by_team_id_excluded<'a>(team_id: &'a str) -> ListByTeamIdExcluded<'a> {
        ListByTeamIdExcluded { team_id }
    }

    /// Constructs a step to cascade-delete a workset.
    pub fn delete_cascade<'a>(id: &'a str) -> DeleteCascade<'a> {
        DeleteCascade { id }
    }

    /// Constructs a step to allocate a comic index.
    pub fn increment_comic_next_index<'a>(id: &'a str) -> IncrementComicNextIndex<'a> {
        IncrementComicNextIndex { id }
    }

    /// Constructs a step to adjust a workset's comic count.
    pub fn update_comic_count<'a>(id: &'a str, delta: i32) -> UpdateComicCount<'a> {
        UpdateComicCount { id, delta }
    }
}
