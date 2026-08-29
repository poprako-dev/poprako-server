//! Internal normalized unit payloads for chapter translation import.

/// One parsed import unit.
#[derive(Debug)]
pub struct UnitTranslationImport {
    /// Import ordering index within the page.
    pub index: usize,

    /// Horizontal coordinate of this unit on the page image.
    pub x_coord: f64,
    /// Vertical coordinate of this unit on the page image.
    pub y_coord: f64,

    /// Whether this unit is a speech bubble contour.
    pub is_bubble: bool,
    /// Original untranslated text from the source material.
    pub main_text: Option<String>,

    /// Translated text content, absent when not yet translated.
    pub translated_text: Option<String>,
    /// Proofread text content, absent when not yet proofread.
    pub proofread_text: Option<String>,
    /// Whether the proofread text has been reviewed and accepted.
    pub is_proofread: bool,
}
