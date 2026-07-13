//! Internal normalized page payloads for chapter translation import.

use serde::Deserialize;

use crate::model::unit_port::{PoprakoUnitImport, UnitTranslationImport};

/// One parsed import page.
pub struct PageTranslationImport {
    pub units: Vec<UnitTranslationImport>,
}

/// PopRaKo JSON import page.
#[derive(Deserialize)]
pub struct PoprakoPageImport {
    pub image_filename: String,
    pub units: Vec<PoprakoUnitImport>,
}
