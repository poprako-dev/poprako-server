//! Data transfer objects for unit import/export port use cases.

use serde::Serialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

/// JSON-safe export object for one page unit.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UnitTranslationExportVal {
    pub unit_id: String,
    pub unit_index: i32,

    pub page_id: String,
    pub page_index: i32,

    pub x_coord: f64,
    pub y_coord: f64,

    pub is_bubble: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translator_id: Option<String>,

    pub is_proofread: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofreader_id: Option<String>,
}
