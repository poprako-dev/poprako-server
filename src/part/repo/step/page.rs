//! Step types for page repository operations.

use poprako_transactional::step::Step;

use crate::model::page::PageInfo;

/// Step that lists pages by chapter.
pub struct ListByChapter<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for ListByChapter<'a> {
    type Output = Vec<PageInfo>;
}

/// Step that clears image state for all pages in a chapter.
pub struct ClearImagesByChapter<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for ClearImagesByChapter<'a> {
    type Output = ();
}

/// Step that deletes pages by chapter.
pub struct DeleteByChapterId<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for DeleteByChapterId<'a> {
    type Output = ();
}

/// Factory for constructing page repository [`Step`] values.
pub struct PageStep;

impl PageStep {
    /// Constructs a step to list pages by chapter.
    pub fn list_by_chapter<'a>(chapter_id: &'a str) -> ListByChapter<'a> {
        ListByChapter { chapter_id }
    }

    /// Constructs a step to clear page image state by chapter.
    pub fn clear_images_by_chapter<'a>(chapter_id: &'a str) -> ClearImagesByChapter<'a> {
        ClearImagesByChapter { chapter_id }
    }

    /// Constructs a step to delete pages by chapter.
    pub fn delete_by_chapter<'a>(chapter_id: &'a str) -> DeleteByChapterId<'a> {
        DeleteByChapterId { chapter_id }
    }
}
