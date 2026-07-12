//! Step types for chapter repository opers.

use std::collections::HashMap;

use poprako_transactional::step::Step;

use crate::model::{chapter_model, unit_model};
use crate::value::chapter::ChapterInclOpt;

/// Step that lists chapters with include options and pagination.
pub struct ListInfos<'a> {
    pub spec: &'a chapter_model::ListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<chapter_model::Info>;
}

/// Step that inserts a new chapter row.
pub struct Create<'a> {
    pub form: &'a chapter_model::Form,
}

impl<'a> Step for Create<'a> {
    type Output = chapter_model::Info;
}

/// Step that fetches a chapter by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
    pub incl_opt: &'a [ChapterInclOpt],
}

impl<'a> Step for GetInfoById<'a> {
    type Output = chapter_model::Info;
}

/// Step that fetches a chapter by ID with a pessimistic lock.
pub struct GetInfoByIdExcluded<'a> {
    pub id: &'a str,
    pub incl_opt: &'a [ChapterInclOpt],
}

impl<'a> Step for GetInfoByIdExcluded<'a> {
    type Output = chapter_model::Info;
}

// /// Step that lists chapters by comic.
// pub struct ListInfosByComicId<'a> {
//     pub comic_id: &'a str,
// }
//
// impl<'a> Step for ListInfosByComicId<'a> {
//     type Output = Vec<ChapterInfo>;
// }
//
// /// Step that lists chapters by comic with a pessimistic lock.
// pub struct ListInfosByComicIdExcluded<'a> {
//     pub comic_id: &'a str,
// }
//
// impl<'a> Step for ListInfosByComicIdExcluded<'a> {
//     type Output = Vec<ChapterInfo>;
// }

/// Step that lists all chapters by comic with a pessimistic lock.
pub struct ListAllInfosByComicIdExcluded<'a> {
    pub comic_id: &'a str,
}

impl<'a> Step for ListAllInfosByComicIdExcluded<'a> {
    type Output = Vec<chapter_model::Info>;
}

/// Step that finds the pinned chapter under a comic.
pub struct FindPinnedInfoByComicId<'a> {
    pub comic_id: &'a str,
    pub incl_opt: &'a [ChapterInclOpt],
}

impl<'a> Step for FindPinnedInfoByComicId<'a> {
    type Output = Option<chapter_model::Info>;
}

/// Step that batch-queries pinned chapters by comic IDs.
pub struct ListPinnedInfosByComicIds<'a> {
    pub comic_ids: &'a [String],
}

impl<'a> Step for ListPinnedInfosByComicIds<'a> {
    type Output = HashMap<String, chapter_model::Info>;
}

/// Step that updates chapter metadata fields.
pub struct UpdateInfo<'a> {
    pub update: &'a chapter_model::InfoUpdate,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

/// Step that updates chapter workflow phase fields.
pub struct UpdateStage<'a> {
    pub update: &'a chapter_model::StageUpdate,
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
    pub delta: unit_model::CounterDelta,
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
    pub fn list_infos<'a>(spec: &'a chapter_model::ListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to insert a new chapter.
    pub fn create<'a>(form: &'a chapter_model::Form) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to fetch a chapter by ID.
    pub fn get_info_by_id<'a>(
        id: &'a str,
        incl_opt: &'a [ChapterInclOpt],
    ) -> GetInfoById<'a> {
        GetInfoById { id, incl_opt }
    }

    /// Constructs a step to fetch a chapter with a pessimistic lock.
    pub fn get_info_by_id_excluded<'a>(
        id: &'a str,
        incl_opt: &'a [ChapterInclOpt],
    ) -> GetInfoByIdExcluded<'a> {
        GetInfoByIdExcluded { id, incl_opt }
    }

    // /// Constructs a step to list chapters by comic.
    // pub fn list_infos_by_comic_id<'a>(comic_id: &'a str, page: Page) -> ListInfosByComicId<'a> {
    //     ListInfosByComicId {
    //         comic_id,
    //         offset: page.offset,
    //         limit: page.limit,
    //     }
    // }

    // /// Constructs a step to list chapters by comic with a pessimistic lock.
    // pub fn list_infos_by_comic_id_excluded<'a>(
    //     comic_id: &'a str,
    //     page: Page,
    // ) -> ListInfosByComicIdExcluded<'a> {
    //     ListInfosByComicIdExcluded {
    //         comic_id,
    //         offset: page.offset,
    //         limit: page.limit,
    //     }
    // }

    /// Constructs a step to list all chapters by comic with a pessimistic lock.
    pub fn list_all_infos_by_comic_id_excluded<'a>(
        comic_id: &'a str,
    ) -> ListAllInfosByComicIdExcluded<'a> {
        ListAllInfosByComicIdExcluded { comic_id }
    }

    /// Constructs a step to find a pinned chapter by comic.
    pub fn find_pinned_info_by_comic_id<'a>(
        comic_id: &'a str,
        incl_opt: &'a [ChapterInclOpt],
    ) -> FindPinnedInfoByComicId<'a> {
        FindPinnedInfoByComicId { comic_id, incl_opt }
    }

    /// Constructs a step to batch-query pinned chapters by comic IDs.
    pub fn list_pinned_infos_by_comic_ids<'a>(
        comic_ids: &'a [String],
    ) -> ListPinnedInfosByComicIds<'a> {
        ListPinnedInfosByComicIds { comic_ids }
    }

    /// Constructs a step to update chapter metadata.
    pub fn update_info<'a>(
        update: &'a chapter_model::InfoUpdate,
    ) -> UpdateInfo<'a> {
        UpdateInfo { update }
    }

    /// Constructs a step to update chapter workflow phases.
    pub fn update_stage<'a>(
        update: &'a chapter_model::StageUpdate,
    ) -> UpdateStage<'a> {
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
        delta: unit_model::CounterDelta,
    ) -> AdjustUnitCounters<'a> {
        AdjustUnitCounters { id, delta }
    }

    /// Constructs a step to unpin other chapters in the same comic.
    pub fn unpin_others<'a>(
        comic_id: &'a str,
        excluded_id: &'a str,
    ) -> UnpinOthers<'a> {
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
