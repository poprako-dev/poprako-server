//! Comic-list-specific DTOs — payload for the comic listing endpoint.

use serde::Serialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use crate::data::assignment::AssignmentInfoVal;
use crate::data::chapter::ChapterInfoVal;
use crate::data::comic::ComicInfoVal;

/// Presentation-ready comic list and optional pinned chapters.
///
/// `pinned_chapters` and `pinned_chapter_assignments` are positionally aligned
/// with `comics`. Their entries are populated only when the corresponding
/// `with` options are requested.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ListComicInfosPayload {
    pub comics: Vec<ComicInfoVal>,
    pub pinned_chapters: Vec<Option<ChapterInfoVal>>,
    pub pinned_chapter_assignments: Vec<Vec<AssignmentInfoVal>>,
}
