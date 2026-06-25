//! Step types for comic repository operations.

use poprako_transactional::step::Step;

use crate::model::comic::{ComicCoverReservation, ComicForm, ComicInfo, ComicInfoUpdate};

/// Step that inserts a new comic row.
pub struct Create<'a> {
    pub form: &'a ComicForm,
}

impl<'a> Step for Create<'a> {
    type Output = ComicInfo;
}

/// Step that fetches a comic by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = ComicInfo;
}

/// Step that fetches a comic by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = ComicInfo;
}

/// Step that lists all comics for a workset.
pub struct ListByWorksetId<'a> {
    pub workset_id: &'a str,
}

impl<'a> Step for ListByWorksetId<'a> {
    type Output = Vec<ComicInfo>;
}

/// Step that lists all comics for a workset with a pessimistic lock.
pub struct ListByWorksetIdExcluded<'a> {
    pub workset_id: &'a str,
}

impl<'a> Step for ListByWorksetIdExcluded<'a> {
    type Output = Vec<ComicInfo>;
}

/// Step that updates a comic's profile fields.
pub struct UpdateInfo<'a> {
    pub update: &'a ComicInfoUpdate,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

/// Step that reserves a new cover upload slot for a comic.
pub struct ReserveCover<'a> {
    pub id: &'a str,
    pub file_extension: &'a str,
}

impl<'a> Step for ReserveCover<'a> {
    type Output = ComicCoverReservation;
}

/// Step that marks a reserved cover as successfully uploaded.
pub struct MarkCoverUploaded<'a> {
    pub id: &'a str,
    pub cover_version: i64,
}

impl<'a> Step for MarkCoverUploaded<'a> {
    type Output = ();
}

/// Step that deletes a comic by its identifier.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Step that marks a comic completed or active.
pub struct MarkCompleted<'a> {
    pub id: &'a str,
    pub is_completed: bool,
}

impl<'a> Step for MarkCompleted<'a> {
    type Output = ();
}

/// Factory for constructing comic repository [`Step`] values.
pub struct ComicStep;

impl ComicStep {
    /// Constructs a step to insert a new comic.
    pub fn create<'a>(form: &'a ComicForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to fetch a comic by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to fetch a comic with a pessimistic lock.
    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    /// Constructs a step to list a workset's comics.
    pub fn list_by_workset_id<'a>(workset_id: &'a str) -> ListByWorksetId<'a> {
        ListByWorksetId { workset_id }
    }

    /// Constructs a step to list a workset's comics with a pessimistic lock.
    pub fn list_by_workset_id_excluded<'a>(
        workset_id: &'a str,
    ) -> ListByWorksetIdExcluded<'a> {
        ListByWorksetIdExcluded { workset_id }
    }

    /// Constructs a step to update a comic's profile fields.
    pub fn update_info<'a>(update: &'a ComicInfoUpdate) -> UpdateInfo<'a> {
        UpdateInfo { update }
    }

    /// Constructs a step to reserve a new comic cover upload slot.
    pub fn reserve_cover<'a>(id: &'a str, file_extension: &'a str) -> ReserveCover<'a> {
        ReserveCover { id, file_extension }
    }

    /// Constructs a step to confirm a comic cover upload completed.
    pub fn mark_cover_uploaded<'a>(id: &'a str, cover_version: i64) -> MarkCoverUploaded<'a> {
        MarkCoverUploaded { id, cover_version }
    }

    /// Constructs a step to delete a comic.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }

    /// Constructs a step to mark completion state.
    pub fn mark_completed<'a>(id: &'a str, is_completed: bool) -> MarkCompleted<'a> {
        MarkCompleted { id, is_completed }
    }
}
