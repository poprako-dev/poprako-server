//! Val DTOs for the chapter port domain.

//! Data transfer objects for chapter import/export port use cases.

use serde::Serialize;

use crate::data::view::page_port::PageTranslationExportView;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// JSON-safe export object for one translated chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportChapterTranslationVal {
    /// Chapter identifier.
    pub chapter_id: String,
    /// Ordinal index of the chapter within its comic.
    pub chapter_index: i32,
    /// Optional subtitle for the chapter; absent when not set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_subtitle: Option<String>,

    /// Identifier of the comic this chapter belongs to.
    pub comic_id: String,
    /// Title of the comic this chapter belongs to.
    pub comic_title: String,

    /// Translated pages contained in this chapter.
    pub pages: Vec<PageTranslationExportView>,
}

/// Summary returned after importing chapter translations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationVal {
    /// Number of pages that were imported.
    pub imported_page_count: i32,
    /// Number of translation units that were imported.
    pub imported_unit_count: i32,
}
