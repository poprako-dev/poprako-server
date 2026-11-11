//! View DTOs for unit translation port import and export.
use crate::value::chapter_port::document::UnitTranslationDocument;

/// Unit object exchanged by the `PopRaKo` translation port.
#[derive(Clone)]
pub struct UnitTranslationPortView {
    /// Unit identifier from the exporting chapter.
    pub unit_id: String,

    /// Ordinal index of the unit within its page.
    pub unit_index: usize,

    /// Parent page identifier from the exporting chapter.
    pub page_id: String,

    /// Ordinal index of the page within its chapter.
    pub page_index: usize,

    /// Horizontal coordinate of the unit bounding box on the page.
    pub x_coord: f64,
    /// Vertical coordinate of the unit bounding box on the page.
    pub y_coord: f64,

    /// Whether this unit represents a speech bubble area.
    pub is_bubble: bool,
    /// Whether this Unit is flagged for later review.
    pub is_flagged: bool,

    /// Translation text annotation delivered to typesetters.
    pub typography_note: Option<String>,
    /// Communication between translators and proofreaders.
    pub translation_note: Option<String>,

    /// Translated text content, or [`None`] if not translated.
    pub translated_text: Option<String>,
    /// Identifier of the translator user, or [`None`]. This is export metadata.
    pub translator_id: Option<String>,

    /// Whether this unit has been proofread.
    pub is_proofread: bool,
    /// Proofread text content, or [`None`] if not proofread.
    pub proofread_text: Option<String>,
    /// Identifier of the proofreader user, or [`None`]. This is export metadata.
    pub proofreader_id: Option<String>,
}

impl From<UnitTranslationDocument> for UnitTranslationPortView {
    // Converts the native exchange document into its response representation.
    fn from(document: UnitTranslationDocument) -> Self {
        Self {
            unit_id: document.unit_id,
            unit_index: document.unit_index,
            page_id: document.page_id,
            page_index: document.page_index,
            x_coord: document.x_coord,
            y_coord: document.y_coord,
            is_bubble: document.is_bubble,
            is_flagged: document.is_flagged,
            typography_note: document.typography_note,
            translation_note: document.translation_note,
            translated_text: document.translated_text,
            translator_id: document.translator_id,
            is_proofread: document.is_proofread,
            proofread_text: document.proofread_text,
            proofreader_id: document.proofreader_id,
        }
    }
}
