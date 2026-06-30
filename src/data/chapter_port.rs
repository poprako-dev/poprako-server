use crate::value::chapter_port::TranslationFormat;

/// NOTE: NO NEED TO DERIVE SERDE.
pub struct ChapterTranslationImportData {
    pub format: TranslationFormat,
    pub content: String,
}
