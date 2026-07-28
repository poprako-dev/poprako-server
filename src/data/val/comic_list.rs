//! Val DTOs for the comic list domain.

//! Comic-list-specific DTOs — payload for the comic listing endpoint.

use serde::Serialize;

use crate::data::val::assignment::AssignmentInfoVal;
use crate::data::val::chapter::ChapterInfoVal;
use crate::data::val::comic::ComicInfoVal;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Presentation-ready comic list and optional pinned chapters.
///
/// `pinned_chapters` and `pinned_chapter_assignments` are positionally aligned
/// with `comics`. Their entries are populated only when the corresponding
/// `with` options are requested.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListComicInfosVal {
    //
    /// Comic information for the listed comics.
    pub comics: Vec<ComicInfoVal>,

    /// Pinned chapter for each comic, positionally aligned with `comics`.
    /// `None` when the comic has no pinned chapter.
    pub pinned_chapters: Vec<Option<ChapterInfoVal>>,

    /// Assignments for each pinned chapter, positionally aligned with `comics`.
    pub pinned_chapter_assignments: Vec<Vec<AssignmentInfoVal>>,
}
