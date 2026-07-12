//! Internal normalized page payloads for chapter translation import.

use serde::Deserialize;

use crate::model::unit_port_model;

/// One parsed import page.
pub struct TranslationImport {
    pub units: Vec<unit_port_model::TranslationImport>,
}

/// PopRaKo JSON import page.
#[derive(Deserialize)]
pub struct PoprakoImport {
    pub image_filename: String,
    pub units: Vec<unit_port_model::PoprakoImport>,
}
