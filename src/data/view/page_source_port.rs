//! View DTOs for page translation port import and export.
use crate::data::view::unit_port::UnitTranslationPortView;
use crate::value::chapter_port::document::PageSourceTranslationDocument;

/// source page object exchanged by the `PopRaKo` translation port.
#[derive(Clone)]
pub struct PageSourceTranslationPortView {
    /// source page identifier from the exporting chapter.
    pub page_id: String,
    /// Zero-based page index within its chapter.
    pub page_index: usize,

    /// Units belonging to this page.
    pub units: Vec<UnitTranslationPortView>,
}

impl From<PageSourceTranslationDocument> for PageSourceTranslationPortView {
    // Converts the native exchange document into its response representation.
    fn from(document: PageSourceTranslationDocument) -> Self {
        Self {
            page_id: document.page_id,
            page_index: document.page_index,
            units: document.units.into_iter().map(Into::into).collect(),
        }
    }
}
