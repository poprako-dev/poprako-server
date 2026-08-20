//! Val DTOs for the comic list domain.

//! Comic-list-specific DTOs — payload for the comic listing endpoint.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::comic::ComicInfoView;

/// Presentation-ready comic list and optional pinned chapters.
///
/// `pinned_chapters` and `pinned_chapter_assignments` are positionally aligned
/// with `comics`. Their entries are populated only when the corresponding
/// `with` options are requested.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListComicInfosVal {
    /// Comic information for the listed comics.
    pub comics: Vec<ComicInfoView>,

    /// Pinned chapter for each comic, positionally aligned with `comics`.
    /// `None` when the comic has no pinned chapter.
    pub pinned_chapters: Vec<Option<ChapterInfoView>>,

    /// Assignments for each pinned chapter, positionally aligned with `comics`.
    pub pinned_chapter_assignments: Vec<Vec<AssignmentInfoView>>,
}
