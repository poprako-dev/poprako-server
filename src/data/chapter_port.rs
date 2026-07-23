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
    pub format: TranslationFormat,
    pub content: String,
}

/// JSON-safe export object for one translated chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportChapterTranslationPayload {
    pub chapter_id: String,
    pub chapter_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_subtitle: Option<String>,

    pub comic_id: String,
    pub comic_title: String,

    pub pages: Vec<PageTranslationExportPayload>,
}

/// Summary returned after importing chapter translations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationPayload {
    pub imported_page_count: i32,
    pub imported_unit_count: i32,
}
