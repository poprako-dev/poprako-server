//! Data transfer objects for chapter import/export port use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::page_port::PageTranslationExportPayload;
use crate::value::chapter_port::TranslationFormat;

/// Request body for importing chapter translations.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationParams {
    //
    /// The translation format (e.g., JSON, SRT).
    pub format: TranslationFormat,
    /// Raw translation content string.
    pub content: String,
}

/// JSON-safe export object for one translated chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportChapterTranslationPayload {
    //
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
    pub pages: Vec<PageTranslationExportPayload>,
}

/// Summary returned after importing chapter translations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationPayload {
    //
    /// Number of pages that were imported.
    pub imported_page_count: i32,
    /// Number of translation units that were imported.
    pub imported_unit_count: i32,
}
