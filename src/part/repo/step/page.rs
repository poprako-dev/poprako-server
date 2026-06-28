//! Step types for page repository operations.

use poprako_transactional::step::Step;

use crate::model::page::PageInfo;

/// Step that lists pages by chapter.
pub struct ListInfosByChapter<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Step for ListInfosByChapter<'a> {
    type Output = Vec<PageInfo>;
}

/// Factory for constructing page repository [`Step`] values.
pub struct PageStep;

impl PageStep {
    /// Constructs a step to list pages by chapter.
    pub fn list_infos_by_chapter<'a>(chapter_id: &'a str) -> ListInfosByChapter<'a> {
        ListInfosByChapter { chapter_id }
    }
}
