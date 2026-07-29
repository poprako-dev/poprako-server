//! Data transfer objects for page import/export port use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::unit_port::UnitTranslationExportPayload;

/// JSON-safe export object for one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageTranslationExportPayload {
    //
    /// Page identifier.
    pub page_id: String,
    /// Zero-based page index within its chapter.
    pub page_index: i32,

    /// Translation units belonging to this page.
    pub units: Vec<UnitTranslationExportPayload>,
}
