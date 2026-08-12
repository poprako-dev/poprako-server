//! View DTOs for the page port domain.

//! Data transfer objects for page import/export port use cases.

use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::unit_port::UnitTranslationExportView;

/// JSON-safe export object for one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageTranslationExportView {
    /// Page identifier.
    pub page_id: String,
    /// Zero-based page index within its chapter.
    pub page_index: i32,

    /// Translation units belonging to this page.
    pub units: Vec<UnitTranslationExportView>,
}
