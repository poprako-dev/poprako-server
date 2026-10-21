//! View DTOs for chapter translation port import and export.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::page_port::PageTranslationPortView;
use crate::model::artifact::translation_document::ChapterTranslationDocument;

/// JSON document exchanged by the `PopRaKo` translation port.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterTranslationPortView {
    /// Chapter identifier from the exporting chapter.
    pub chapter_id: String,
    /// Ordinal index of the chapter within its comic.
    pub chapter_index: usize,
    /// Optional subtitle from the exporting chapter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_subtitle: Option<String>,

    /// Comic identifier from the exporting comic.
    pub comic_id: String,
    /// Comic title from the exporting comic.
    pub comic_title: String,

    /// Pages and their translation units.
    pub pages: Vec<PageTranslationPortView>,
}

impl From<ChapterTranslationDocument> for ChapterTranslationPortView {
    // Converts the native exchange document into its response representation.
    fn from(document: ChapterTranslationDocument) -> Self {
        Self {
            chapter_id: document.chapter_id,
            chapter_index: document.chapter_index,
            chapter_subtitle: document.chapter_subtitle,
            comic_id: document.comic_id,
            comic_title: document.comic_title,
            pages: document.pages.into_iter().map(Into::into).collect(),
        }
    }
}

/// A direct artwork upload capability.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterArtworkUploadSlotView {
    /// Presigned PUT URL.
    pub put_url: String,
    /// Headers bound into the upload signature.
    pub headers: BTreeMap<String, String>,
}
