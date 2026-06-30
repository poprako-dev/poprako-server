//! Data transfer objects for page import/export port use cases.

use crate::data::unit_port::UnitTranslationExportVal;

/// JSON-safe export object for one page.
pub struct PageTranslationExportVal {
    pub page_id: String,
    pub page_index: i32,

    pub image_url: Option<String>,
    pub is_uploaded: bool,

    pub units: Vec<UnitTranslationExportVal>,
}
