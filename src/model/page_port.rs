//! Internal normalized page payloads for chapter translation import.

use crate::model::unit_port::UnitTranslationImport;

/// One parsed import page.
#[derive(Debug)]
pub struct PageTranslationImport {
    //
    /// Zero-based page index in the imported document.
    pub page_index: usize,

    /// Translated units belonging to this imported page.
    pub units: Vec<UnitTranslationImport>,
}
