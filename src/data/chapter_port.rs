use crate::data::page_port::PageTranslationExportVal;
use crate::value::chapter_port::TranslationFormat;

/// NOTE: NO NEED TO DERIVE SERDE.
pub struct ChapterTranslationImportData {
    pub format: TranslationFormat,
    pub content: String,
}

/// JSON-safe export object for one translated chapter.
pub struct ChapterTranslationExportVal {
    pub chapter_id: String,
    pub chapter_index: i32,
    pub chapter_subtitle: Option<String>,

    pub comic_id: String,
    pub comic_title: String,

    pub pages: Vec<PageTranslationExportVal>,
}

/// Summary returned after importing chapter translations.
pub struct ChapterTranslationImportVal {
    pub imported_page_count: i32,
    pub imported_unit_count: i32,
}
