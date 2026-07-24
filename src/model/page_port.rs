//! Internal normalized page payloads for chapter translation import.

use serde::Deserialize;

use crate::model::unit_port::{PoprakoUnitImport, UnitTranslationImport};

/// One parsed import page.
pub struct PageTranslationImport {
    /// Translated units belonging to this imported page.
    pub units: Vec<UnitTranslationImport>,
}

/// PopRaKo JSON import page.
#[derive(Deserialize)]
pub struct PoprakoPageImport {
    //
    /// Filename of the page image from the import archive.
    pub image_filename: String,
    /// Import units belonging to this page in PopRaKo format.
    pub units: Vec<PoprakoUnitImport>,
}
