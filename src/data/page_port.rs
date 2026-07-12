//! Data transfer objects for page import/export port use cases.

use serde::Serialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use crate::data::unit_port_data;

/// JSON-safe export object for one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct TranslationExportVal {
    pub page_id: String,
    pub page_index: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    pub units: Vec<unit_port_data::TranslationExportVal>,
}
