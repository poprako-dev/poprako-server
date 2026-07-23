//! Data transfer objects for page import/export port use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::unit_port::UnitTranslationExportPayload;

/// JSON-safe export object for one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageTranslationExportPayload {
    pub page_id: String,
    pub page_index: i32,

    pub units: Vec<UnitTranslationExportPayload>,
}
