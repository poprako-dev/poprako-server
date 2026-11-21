//! HTTP wire objects for the migrated data DTOs.
use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::object::assignment::AssignmentInfoObj;
use crate::api::http::object::chapter::ChapterInfoObj;
use crate::api::http::object::comic::ComicInfoObj;
use crate::api::http::result::Wire;
use crate::data::val::comic_list::ListComicInfosVal;

/// Presentation-ready comic list and optional pinned chapters.
///
/// `pinned_chapters` and `pinned_chapter_assignments` are positionally aligned
/// with `comics`. Their entries are populated only when the corresponding
/// `with` options are requested.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListComicInfosPayload {
    /// Comic information for the listed comics.
    pub comics: Vec<ComicInfoObj>,

    /// Pinned chapter for each comic, positionally aligned with `comics`.
    /// `None` when the comic has no pinned chapter.
    pub pinned_chapters: Vec<Option<ChapterInfoObj>>,

    /// Assignments, including their users, for each pinned chapter,
    /// positionally aligned with `comics`.
    pub pinned_chapter_assignments: Vec<Vec<AssignmentInfoObj>>,
}

impl From<ListComicInfosVal> for ListComicInfosPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListComicInfosVal) -> Self {
        Self {
            comics: value
                .comics
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
            pinned_chapters: value
                .pinned_chapters
                .into_iter()
                .map(|item| item.map(std::convert::Into::into))
                .collect(),
            pinned_chapter_assignments: value
                .pinned_chapter_assignments
                .into_iter()
                .map(|item| {
                    item.into_iter().map(std::convert::Into::into).collect()
                })
                .collect(),
        }
    }
}

impl Wire for ListComicInfosVal {
    // Serializable wire payload produced by this conversion.
    type Target = ListComicInfosPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
