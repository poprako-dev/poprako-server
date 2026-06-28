//! Step types for chapter repository operations.

use poprako_transactional::step::Step;

use crate::model::chapter::{ChapterForm, ChapterInfo, ChapterInfoUpdate, ChapterStageUpdate};

/// Step that inserts a new chapter row.
pub struct Create<'a> {
    pub form: &'a ChapterForm,
}

impl<'a> Step for Create<'a> {
    type Output = ChapterInfo;
}

/// Step that fetches a chapter by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = ChapterInfo;
}

/// Step that fetches a chapter by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = ChapterInfo;
}

/// Step that lists chapters by comic.
pub struct ListInfosByComicId<'a> {
    pub comic_id: &'a str,
    pub offset: u64,
    pub limit: u64,
}

impl<'a> Step for ListInfosByComicId<'a> {
    type Output = Vec<ChapterInfo>;
}

/// Step that lists chapters by comic with a pessimistic lock.
pub struct ListInfosByComicIdExcluded<'a> {
    pub comic_id: &'a str,
    pub offset: u64,
    pub limit: u64,
}

impl<'a> Step for ListInfosByComicIdExcluded<'a> {
    type Output = Vec<ChapterInfo>;
}

/// Step that lists all chapters by comic with a pessimistic lock.
pub struct ListAllInfosByComicIdExcluded<'a> {
    pub comic_id: &'a str,
}

impl<'a> Step for ListAllInfosByComicIdExcluded<'a> {
    type Output = Vec<ChapterInfo>;
}

/// Step that finds the pinned chapter under a comic.
pub struct FindPinnedInfoByComicId<'a> {
    pub comic_id: &'a str,
}

impl<'a> Step for FindPinnedInfoByComicId<'a> {
    type Output = Option<ChapterInfo>;
}

/// Step that updates chapter metadata fields.
pub struct UpdateInfo<'a> {
    pub update: &'a ChapterInfoUpdate,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

/// Step that updates chapter workflow phase fields.
pub struct UpdateStage<'a> {
    pub update: &'a ChapterStageUpdate,
}

impl<'a> Step for UpdateStage<'a> {
    type Output = ();
}

/// Step that unpins all other chapters under a comic.
pub struct UnpinOthers<'a> {
    pub comic_id: &'a str,
    pub excluded_id: &'a str,
}

impl<'a> Step for UnpinOthers<'a> {
    type Output = ();
}

/// Step that deletes a chapter by its identifier.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Factory for constructing chapter repository [`Step`] values.
pub struct ChapterStep;

impl ChapterStep {
    /// Constructs a step to insert a new chapter.
    pub fn create<'a>(form: &'a ChapterForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to fetch a chapter by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to fetch a chapter with a pessimistic lock.
    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    /// Constructs a step to list chapters by comic.
    pub fn list_infos_by_comic_id<'a>(
        comic_id: &'a str,
        offset: u64,
        limit: u64,
    ) -> ListInfosByComicId<'a> {
        ListInfosByComicId {
            comic_id,
            offset,
            limit,
        }
    }

    /// Constructs a step to list chapters by comic with a pessimistic lock.
    pub fn list_infos_by_comic_id_excluded<'a>(
        comic_id: &'a str,
        offset: u64,
        limit: u64,
    ) -> ListInfosByComicIdExcluded<'a> {
        ListInfosByComicIdExcluded {
            comic_id,
            offset,
            limit,
        }
    }

    /// Constructs a step to list all chapters by comic with a pessimistic lock.
    pub fn list_all_infos_by_comic_id_excluded<'a>(
        comic_id: &'a str,
    ) -> ListAllInfosByComicIdExcluded<'a> {
        ListAllInfosByComicIdExcluded { comic_id }
    }

    /// Constructs a step to find a pinned chapter by comic.
    pub fn find_pinned_info_by_comic_id<'a>(comic_id: &'a str) -> FindPinnedInfoByComicId<'a> {
        FindPinnedInfoByComicId { comic_id }
    }

    /// Constructs a step to update chapter metadata.
    pub fn update_info<'a>(update: &'a ChapterInfoUpdate) -> UpdateInfo<'a> {
        UpdateInfo { update }
    }

    /// Constructs a step to update chapter workflow phases.
    pub fn update_stage<'a>(update: &'a ChapterStageUpdate) -> UpdateStage<'a> {
        UpdateStage { update }
    }

    /// Constructs a step to unpin other chapters in the same comic.
    pub fn unpin_others<'a>(comic_id: &'a str, excluded_id: &'a str) -> UnpinOthers<'a> {
        UnpinOthers {
            comic_id,
            excluded_id,
        }
    }

    /// Constructs a step to delete a chapter.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
