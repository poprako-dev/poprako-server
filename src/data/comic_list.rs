//! Comic-list-specific DTOs — payload for the comic listing endpoint.

use serde::Serialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use crate::data::chapter::ChapterInfoVal;
use crate::data::comic::ComicInfoVal;

/// Presentation-ready comic list and optional pinned chapters.
///
/// `pinned_chapters` is positionally aligned with `comics`. Its entries are
/// populated only when the request includes `with=pinned_chapter`.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ListComicInfosPayload {
    pub comics: Vec<ComicInfoVal>,
    pub pinned_chapters: Vec<Option<ChapterInfoVal>>,
}
