//! Step types for page repository opers.

use poprako_macro::Paginate;
use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::{page_model, unit_model};

/// Step that fetches a page by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = page_model::Info;
}

/// Step that fetches a page by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = page_model::Info;
}

/// Step that lists pages by chapter ID.
#[Paginate]
pub struct ListInfosByChapterId<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for ListInfosByChapterId<'a> {
    type Output = Vec<page_model::Info>;
}

/// Step that lists all pages by chapter ID.
pub struct ListAllInfosByChapterId<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for ListAllInfosByChapterId<'a> {
    type Output = Vec<page_model::Info>;
}

/// Step that inserts multiple page rows.
pub struct CreateBatch<'a> {
    pub forms: &'a [page_model::Form],
}

impl<'a> Step for CreateBatch<'a> {
    type Output = Vec<page_model::Info>;
}

/// Step that reserves a page image key and resets upload state.
pub struct ReserveImage<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Step for ReserveImage<'a> {
    type Output = page_model::ImageReservation;
}

/// Step that marks a page image upload as completed.
pub struct MarkImageUploaded<'a> {
    pub id: &'a str,
    pub image_version: i64,
}

impl<'a> Step for MarkImageUploaded<'a> {
    type Output = ();
}

/// Step that overwrites unit counters for one page.
pub struct SetUnitCounters<'a> {
    pub id: &'a str,
    pub counters: unit_model::Counters,
}

impl<'a> Step for SetUnitCounters<'a> {
    type Output = ();
}

/// Step that deletes all pages under one chapter.
pub struct DeleteByChapterId<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for DeleteByChapterId<'a> {
    type Output = ();
}

/// Factory for constructing page repository [`Step`] values.
pub struct PageStep;

impl PageStep {
    /// Constructs a step to fetch a page by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to fetch a page with a pessimistic lock.
    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    /// Constructs a step to list pages by chapter ID.
    pub fn list_infos_by_chapter_id<'a>(
        chapter_id: &'a str,
        page: Page,
    ) -> ListInfosByChapterId<'a> {
        ListInfosByChapterId {
            chapter_id,
            offset: page.offset,
            limit: page.limit,
        }
    }

    /// Constructs a step to list all pages by chapter ID.
    pub fn list_all_infos_by_chapter_id<'a>(
        chapter_id: &'a str,
    ) -> ListAllInfosByChapterId<'a> {
        ListAllInfosByChapterId { chapter_id }
    }

    /// Constructs a step to insert multiple pages.
    pub fn create_batch<'a>(forms: &'a [page_model::Form]) -> CreateBatch<'a> {
        CreateBatch { forms }
    }

    /// Constructs a step to reserve one page image.
    pub fn reserve_image<'a>(
        id: &'a str,
        file_ext: &'a str,
    ) -> ReserveImage<'a> {
        ReserveImage { id, file_ext }
    }

    /// Constructs a step to mark a page image uploaded.
    pub fn mark_image_uploaded<'a>(
        id: &'a str,
        image_version: i64,
    ) -> MarkImageUploaded<'a> {
        MarkImageUploaded { id, image_version }
    }

    /// Constructs a step to overwrite unit counters for one page.
    pub fn set_unit_counters<'a>(
        id: &'a str,
        counters: unit_model::Counters,
    ) -> SetUnitCounters<'a> {
        SetUnitCounters { id, counters }
    }

    /// Constructs a step to delete all pages under one chapter.
    pub fn delete_by_chapter_id<'a>(
        chapter_id: &'a str,
    ) -> DeleteByChapterId<'a> {
        DeleteByChapterId { chapter_id }
    }
}
