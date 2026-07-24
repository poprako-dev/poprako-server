//! Data transfer objects for unit import/export port use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// JSON-safe export object for one page unit.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitTranslationExportPayload {
    /// Unique unit identifier for the exported unit.
    pub unit_id: String,
    /// Ordinal index of the unit within its page.
    pub unit_index: i32,

    /// Parent page identifier the unit belongs to.
    pub page_id: String,
    /// Ordinal index of the page within its chapter.
    pub page_index: i32,

    /// Horizontal coordinate of the unit bounding box on the page.
    pub x_coord: f64,
    /// Vertical coordinate of the unit bounding box on the page.
    pub y_coord: f64,

    /// Whether this unit represents a speech bubble area.
    pub is_bubble: bool,

    /// Translated text content, or [`None`] if not translated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// Identifier of the translator user, or [`None`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translator_id: Option<String>,

    /// Whether this unit has been proofread.
    pub is_proofread: bool,
    /// Proofread text content, or [`None`] if not proofread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    /// Identifier of the proofreader user, or [`None`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofreader_id: Option<String>,
}
