//! Step types for team repository opers.

use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::team_model;

/// Step that inserts a new team row.
pub struct Create<'a> {
    pub form: &'a team_model::Form,
}

impl<'a> Step for Create<'a> {
    type Output = team_model::Info;
}

/// Step that fetches a team by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = team_model::Info;
}

/// Step that lists teams with pagination.
pub struct ListInfos<'a> {
    pub user_id: Option<&'a str>,

    pub offset: u32,
    pub limit: u32,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<team_model::Info>;
}

/// Step that updates a team's name and description.
pub struct UpdateInfo<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

/// Step that reserves a new avatar upload slot for a team.
pub struct ReserveAvatar<'a> {
    pub id: &'a str,
    pub file_extension: &'a str,
}

impl<'a> Step for ReserveAvatar<'a> {
    type Output = team_model::AvatarReservation;
}

/// Step that marks a reserved team avatar as successfully uploaded.
pub struct MarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: u32,
}

impl<'a> Step for MarkAvatarUploaded<'a> {
    type Output = ();
}

/// Step that fetches a team by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = team_model::Info;
}

/// Step that deletes a team by its identifier.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Step that allocates one workset index from a team-scoped sequence.
///
/// NOTE: Return the current `workset_next_index` value, then increment it in
/// the same transactional write. A storage implementation can satisfy this
/// with one atomic `UPDATE ... SET workset_next_index = workset_next_index + 1
/// RETURNING workset_next_index - 1`.
pub struct IncrementWorksetNextIndex<'a> {
    pub id: &'a str,
}

impl<'a> Step for IncrementWorksetNextIndex<'a> {
    type Output = i32;
}

/// Factory for constructing team repository [`Step`] values.
pub struct TeamStep;

impl TeamStep {
    /// Constructs a step to insert a new team.
    pub fn create<'a>(form: &'a team_model::Form) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to fetch a team by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to list teams with optional user scoping.
    pub fn list_infos<'a>(
        user_id: Option<&'a str>,
        page: Page,
    ) -> ListInfos<'a> {
        ListInfos {
            user_id,
            offset: page.offset,
            limit: page.limit,
        }
    }

    /// Constructs a step to update a team's name and description.
    pub fn update_info<'a>(
        id: &'a str,
        name: &'a str,
        description: &'a str,
    ) -> UpdateInfo<'a> {
        UpdateInfo {
            id,
            name,
            description,
        }
    }

    /// Constructs a step to reserve a new team avatar upload slot.
    pub fn reserve_avatar<'a>(
        id: &'a str,
        file_extension: &'a str,
    ) -> ReserveAvatar<'a> {
        ReserveAvatar { id, file_extension }
    }

    /// Constructs a step to confirm a team avatar upload completed.
    pub fn mark_avatar_uploaded<'a>(
        id: &'a str,
        avatar_version: u32,
    ) -> MarkAvatarUploaded<'a> {
        MarkAvatarUploaded { id, avatar_version }
    }

    /// Constructs a step to fetch a team with a pessimistic lock.
    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    /// Constructs a step to delete a team.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }

    /// Constructs a step to allocate a workset index.
    pub fn increment_workset_next_index<'a>(
        id: &'a str,
    ) -> IncrementWorksetNextIndex<'a> {
        IncrementWorksetNextIndex { id }
    }
}
