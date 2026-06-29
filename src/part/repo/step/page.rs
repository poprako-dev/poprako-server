//! Step types for page repository operations.

use poprako_transactional::step::Step;

use crate::model::page::{PageForm, PageImageReservation, PageInfo};

/// Step that fetches a page by its identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = PageInfo;
}

/// Step that fetches a page by ID with a pessimistic lock.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = PageInfo;
}

/// Step that lists pages by chapter.
pub struct ListInfosByChapter<'a> {
    pub chapter_id: &'a str,
    pub offset: u64,
    pub limit: u64,
}

impl<'a> Step for ListInfosByChapter<'a> {
    type Output = Vec<PageInfo>;
}

/// Step that lists all pages by chapter.
pub struct ListAllInfosByChapter<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for ListAllInfosByChapter<'a> {
    type Output = Vec<PageInfo>;
}

/// Step that inserts multiple page rows.
pub struct CreateBatch<'a> {
    pub forms: &'a [PageForm],
}

impl<'a> Step for CreateBatch<'a> {
    type Output = Vec<PageInfo>;
}

/// Step that reserves a page image key and resets upload state.
pub struct ReserveImage<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Step for ReserveImage<'a> {
    type Output = PageImageReservation;
}

/// Step that marks a page image upload as completed.
pub struct MarkImageUploaded<'a> {
    pub id: &'a str,
    pub image_version: i64,
}

impl<'a> Step for MarkImageUploaded<'a> {
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

    /// Constructs a step to list pages by chapter.
    pub fn list_infos_by_chapter<'a>(
        chapter_id: &'a str,
        offset: u64,
        limit: u64,
    ) -> ListInfosByChapter<'a> {
        ListInfosByChapter {
            chapter_id,
            offset,
            limit,
        }
    }

    /// Constructs a step to list all pages by chapter.
    pub fn list_all_infos_by_chapter<'a>(chapter_id: &'a str) -> ListAllInfosByChapter<'a> {
        ListAllInfosByChapter { chapter_id }
    }

    /// Constructs a step to insert multiple pages.
    pub fn create_batch<'a>(forms: &'a [PageForm]) -> CreateBatch<'a> {
        CreateBatch { forms }
    }

    /// Constructs a step to reserve one page image.
    pub fn reserve_image<'a>(id: &'a str, file_ext: &'a str) -> ReserveImage<'a> {
        ReserveImage { id, file_ext }
    }

    /// Constructs a step to mark a page image uploaded.
    pub fn mark_image_uploaded<'a>(id: &'a str, image_version: i64) -> MarkImageUploaded<'a> {
        MarkImageUploaded { id, image_version }
    }

    /// Constructs a step to delete all pages under one chapter.
    pub fn delete_by_chapter_id<'a>(chapter_id: &'a str) -> DeleteByChapterId<'a> {
        DeleteByChapterId { chapter_id }
    }
}
