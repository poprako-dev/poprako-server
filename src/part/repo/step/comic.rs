//! Step types for comic repository opers.

use poprako_transactional::step::Step;

use crate::model::comic::{
    ComicCoverReservation, ComicForm, ComicInfo, ComicInfoUpdate, ComicListSpec,
};
use crate::value::comic::ComicInclOpt;

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
    pub incl_opt: &'a [ComicInclOpt],
}

impl<'a> Step for GetInfoById<'a> {
    type Output = ComicInfo;
}

/// Step that fetches a comic by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
    pub incl_opt: &'a [ComicInclOpt],
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = ComicInfo;
}

/// Step that lists comics for a workset with filters and pagination.
pub struct ListInfos<'a> {
    pub spec: &'a ComicListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<ComicInfo>;
}

/// Step that lists comics for a workset with a pessimistic lock.
pub struct ListInfosExcluded<'a> {
    pub spec: &'a ComicListSpec,
}

impl<'a> Step for ListInfosExcluded<'a> {
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

/// Step that allocates one chapter index from a comic-scoped sequence.
///
/// NOTE: Return the current `chapter_next_index` value, then increment it in
/// the same transactional write. A storage implementation can satisfy this
/// with one atomic `UPDATE ... SET chapter_next_index = chapter_next_index + 1
/// RETURNING chapter_next_index - 1`.
pub struct IncrChapterNextIndex<'a> {
    pub id: &'a str,
}

impl<'a> Step for IncrChapterNextIndex<'a> {
    type Output = i32;
}

/// Step that changes a comic's chapter count by a delta.
pub struct UpdateChapterCount<'a> {
    pub id: &'a str,
    pub delta: i32,
}

impl<'a> Step for UpdateChapterCount<'a> {
    type Output = ();
}

/// Step that updates a comic's last-active timestamp.
pub struct TouchLastActive<'a> {
    pub id: &'a str,
}

impl<'a> Step for TouchLastActive<'a> {
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
    pub fn get_info_by_id<'a>(
        id: &'a str,
        incl_opt: &'a [ComicInclOpt],
    ) -> GetInfoById<'a> {
        GetInfoById { id, incl_opt }
    }

    /// Constructs a step to fetch a comic with a pessimistic lock.
    pub fn get_info_excluded<'a>(
        id: &'a str,
        incl_opt: &'a [ComicInclOpt],
    ) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id, incl_opt }
    }

    /// Constructs a step to list comics with filters and pagination.
    pub fn list_infos<'a>(spec: &'a ComicListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to list comics with a pessimistic lock.
    pub fn list_infos_excluded<'a>(
        spec: &'a ComicListSpec,
    ) -> ListInfosExcluded<'a> {
        ListInfosExcluded { spec }
    }

    /// Constructs a step to update a comic's profile fields.
    pub fn update_info<'a>(update: &'a ComicInfoUpdate) -> UpdateInfo<'a> {
        UpdateInfo { update }
    }

    /// Constructs a step to reserve a new comic cover upload slot.
    pub fn reserve_cover<'a>(
        id: &'a str,
        file_extension: &'a str,
    ) -> ReserveCover<'a> {
        ReserveCover { id, file_extension }
    }

    /// Constructs a step to confirm a comic cover upload completed.
    pub fn mark_cover_uploaded<'a>(
        id: &'a str,
        cover_version: i64,
    ) -> MarkCoverUploaded<'a> {
        MarkCoverUploaded { id, cover_version }
    }

    /// Constructs a step to delete a comic.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }

    /// Constructs a step to mark completion state.
    pub fn mark_completed<'a>(
        id: &'a str,
        is_completed: bool,
    ) -> MarkCompleted<'a> {
        MarkCompleted { id, is_completed }
    }

    /// Constructs a step to increment and return chapter index.
    pub fn incr_chapter_next_index<'a>(
        id: &'a str,
    ) -> IncrChapterNextIndex<'a> {
        IncrChapterNextIndex { id }
    }

    /// Constructs a step to change chapter count.
    pub fn update_chapter_count<'a>(
        id: &'a str,
        delta: i32,
    ) -> UpdateChapterCount<'a> {
        UpdateChapterCount { id, delta }
    }

    /// Constructs a step to touch comic last-active time.
    pub fn touch_last_active<'a>(id: &'a str) -> TouchLastActive<'a> {
        TouchLastActive { id }
    }
}
