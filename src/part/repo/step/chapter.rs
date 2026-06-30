//! Step types for chapter repository opers.

use poprako_macro::Paginate;
use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::chapter::{
    ChapterForm, ChapterInfo, ChapterInfoUpdate, ChapterListSpec, ChapterStageUpdate,
};
use crate::model::unit::UnitCounterDelta;

/// Step that lists chapters with include options and pagination.
pub struct ListInfos<'a> {
    pub spec: &'a ChapterListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<ChapterInfo>;
}

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
pub struct GetInfoByIdExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoByIdExcluded<'a> {
    type Output = ChapterInfo;
}

/// Step that lists chapters by comic.
#[Paginate]
pub struct ListInfosByComicId<'a> {
    pub comic_id: &'a str,
}

impl<'a> Step for ListInfosByComicId<'a> {
    type Output = Vec<ChapterInfo>;
}

/// Step that lists chapters by comic with a pessimistic lock.
#[Paginate]
pub struct ListInfosByComicIdExcluded<'a> {
    pub comic_id: &'a str,
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

/// Step that overwrites page and unit counters for one chapter.
pub struct SetPageCounters<'a> {
    pub id: &'a str,
    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

impl<'a> Step for SetPageCounters<'a> {
    type Output = ();
}

/// Step that adjusts unit counters for one chapter by delta.
pub struct AdjustUnitCounters<'a> {
    pub id: &'a str,
    pub delta: UnitCounterDelta,
}

impl<'a> Step for AdjustUnitCounters<'a> {
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
    /// Constructs a step to list chapters with include options and pagination.
    pub fn list_infos<'a>(spec: &'a ChapterListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to insert a new chapter.
    pub fn create<'a>(form: &'a ChapterForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to fetch a chapter by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to fetch a chapter with a pessimistic lock.
    pub fn get_info_by_id_excluded<'a>(id: &'a str) -> GetInfoByIdExcluded<'a> {
        GetInfoByIdExcluded { id }
    }

    /// Constructs a step to list chapters by comic.
    pub fn list_infos_by_comic_id<'a>(
        comic_id: &'a str,
        page: Page,
    ) -> ListInfosByComicId<'a> {
        ListInfosByComicId {
            comic_id,
            offset: page.offset,
            limit: page.limit,
        }
    }

    /// Constructs a step to list chapters by comic with a pessimistic lock.
    pub fn list_infos_by_comic_id_excluded<'a>(
        comic_id: &'a str,
        page: Page,
    ) -> ListInfosByComicIdExcluded<'a> {
        ListInfosByComicIdExcluded {
            comic_id,
            offset: page.offset,
            limit: page.limit,
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

    /// Constructs a step to overwrite page and unit counters.
    pub fn set_page_counters<'a>(
        id: &'a str,
        page_count: i32,
        total_unit_count: i32,
        translated_unit_count: i32,
        proofread_unit_count: i32,
    ) -> SetPageCounters<'a> {
        SetPageCounters {
            id,
            page_count,
            total_unit_count,
            translated_unit_count,
            proofread_unit_count,
        }
    }

    /// Constructs a step to adjust unit counters by delta.
    pub fn adjust_unit_counters<'a>(
        id: &'a str,
        delta: UnitCounterDelta,
    ) -> AdjustUnitCounters<'a> {
        AdjustUnitCounters { id, delta }
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
