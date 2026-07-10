//! Data transfer objects for chapter import/export port use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use crate::data::page_port::PageTranslationExportVal;
use crate::value::chapter_port::TranslationFormat;

/// Request body for importing chapter translations.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ChapterTranslationImportData {
    pub format: TranslationFormat,
    pub content: String,
}

/// JSON-safe export object for one translated chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ChapterTranslationExportVal {
    pub chapter_id: String,
    pub chapter_index: i32,
    pub chapter_subtitle: Option<String>,

    pub comic_id: String,
    pub comic_title: String,

    pub pages: Vec<PageTranslationExportVal>,
}

/// Summary returned after importing chapter translations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ChapterTranslationImportVal {
    pub imported_page_count: i32,
    pub imported_unit_count: i32,
}
