//! Internal normalized payloads for chapter translation import.

/// Format-specific source state for one imported unit.
#[derive(Debug)]
pub enum UnitTranslationImportSource {
    //
    /// `LabelPlus` stores one text payload without workflow-stage metadata.
    LabelPlus {
        /// Imported `LabelPlus` text.
        text: Option<String>,
    },

    /// `PopRaKo` stores translation and proofreading state separately.
    PopRaKo {
        /// Translated text content, absent when not yet translated.
        translated_text: Option<String>,
        /// Proofread text content, absent when not yet proofread.
        proofread_text: Option<String>,
        /// Whether the proofread text has been reviewed and accepted.
        is_proofread: bool,
    },
}

/// One parsed import page.
#[derive(Debug)]
pub struct PageTranslationImport {
    //
    /// Zero-based page index in the imported document.
    pub page_index: usize,

    /// Translated units belonging to this imported page.
    pub units: Vec<UnitTranslationImport>,
}

/// One parsed import unit.
#[derive(Debug)]
pub struct UnitTranslationImport {
    //
    /// Import ordering index within the page.
    pub index: usize,

    /// Horizontal coordinate of this unit on the page image.
    pub x_coord: f64,
    /// Vertical coordinate of this unit on the page image.
    pub y_coord: f64,

    /// Whether this unit is a speech bubble contour.
    pub is_bubble: bool,

    /// Format-specific source text and workflow state.
    pub source: UnitTranslationImportSource,
}
