//! Step types for workset repository opers.

use poprako_macro::Paginate;
use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::workset_model;

/// Step that inserts a new workset row.
pub struct Create<'a> {
    pub form: &'a workset_model::Form,
}

impl<'a> Step for Create<'a> {
    type Output = workset_model::Info;
}

/// Step that fetches a workset by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = workset_model::Info;
}

/// Step that lists all worksets for a team with pagination.
#[Paginate]
pub struct ListInfosByTeamId<'a> {
    pub team_id: &'a str,
}

impl<'a> Step for ListInfosByTeamId<'a> {
    type Output = Vec<workset_model::Info>;
}

/// Step that updates a workset's profile fields.
pub struct UpdateInfo<'a> {
    pub update: &'a workset_model::InfoUpdate,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

/// Step that lists all worksets for a team with a pessimistic lock.
pub struct ListAllInfosByTeamIdExcluded<'a> {
    pub team_id: &'a str,
}

impl<'a> Step for ListAllInfosByTeamIdExcluded<'a> {
    type Output = Vec<workset_model::Info>;
}

/// Step that fetches a workset by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = workset_model::Info;
}

/// Step that deletes a workset row.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Step that allocates one comic index from a workset-scoped sequence.
///
/// # Semantics (must be strictly observed)
///
/// - Return the current `comic_next_index` value;
/// - Increment `comic_next_index` by 1 in the same transactional write;
/// - Concurrent callers for the same workset must serialize on the parent row.
///
/// NOTE: A storage implementation can satisfy this with one atomic
/// `UPDATE ... SET comic_next_index = comic_next_index + 1 RETURNING
/// comic_next_index - 1`; do not split allocation into a read followed by an
/// update unless the read locks the parent row.
pub struct IncrComicNextIndex<'a> {
    pub id: &'a str,
}

impl<'a> Step for IncrComicNextIndex<'a> {
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
    pub fn create<'a>(form: &'a workset_model::Form) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to fetch a workset by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to list a team's worksets.
    pub fn list_infos_by_team_id<'a>(
        team_id: &'a str,
        page: Page,
    ) -> ListInfosByTeamId<'a> {
        ListInfosByTeamId {
            team_id,
            offset: page.offset,
            limit: page.limit,
        }
    }

    /// Constructs a step to update a workset's profile fields.
    pub fn update_info<'a>(
        update: &'a workset_model::InfoUpdate,
    ) -> UpdateInfo<'a> {
        UpdateInfo { update }
    }

    /// Constructs a step to list a team's worksets with a pessimistic lock.
    pub fn list_all_infos_by_team_id_excluded<'a>(
        team_id: &'a str,
    ) -> ListAllInfosByTeamIdExcluded<'a> {
        ListAllInfosByTeamIdExcluded { team_id }
    }

    /// Constructs a step to fetch a workset with a pessimistic lock.
    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    /// Constructs a step to delete a workset.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }

    /// Constructs a step to allocate a comic index.
    pub fn incr_comic_next_index<'a>(id: &'a str) -> IncrComicNextIndex<'a> {
        IncrComicNextIndex { id }
    }

    /// Constructs a step to adjust a workset's comic count.
    pub fn update_comic_count<'a>(
        id: &'a str,
        delta: i32,
    ) -> UpdateComicCount<'a> {
        UpdateComicCount { id, delta }
    }
}
