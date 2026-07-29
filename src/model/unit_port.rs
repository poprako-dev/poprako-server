//! Internal normalized unit payloads for chapter translation import.

use serde::Deserialize;

/// One parsed import unit.
pub struct UnitTranslationImport {
    /// Optional server identifier; absent for brand-new imports.
    pub id: Option<String>,
    /// Import ordering index within the page.
    pub index: i32,
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

/// PopRaKo JSON import unit.
#[derive(Deserialize)]
pub struct PoprakoUnitImport {
    /// Server-assigned identifier for the imported unit.
    pub id: String,

    /// Horizontal coordinate of this unit on the page image.
    pub x: f64,
    /// Vertical coordinate of this unit on the page image.
    pub y: f64,
    /// Zero-based display ordering index within the page.
    pub index_in_page: i32,
    /// Whether this unit occupies an inbox-style text box.
    pub is_inbox: bool,

    /// Translated text content, absent when not yet translated.
    pub translated_text: Option<String>,
    /// Proofread (reviewed) text content, absent when not yet proofread.
    pub prooved_text: Option<String>,
    /// Whether the proofread text has been reviewed and accepted.
    pub is_prooved: bool,
}
