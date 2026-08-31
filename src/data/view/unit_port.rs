//! View DTOs for unit translation port import and export.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Unit object exchanged by the `PopRaKo` translation port.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitTranslationPortView {
    //
    /// Unit identifier from the exporting chapter.
    pub unit_id: String,

    /// Ordinal index of the unit within its page.
    pub unit_index: usize,

    /// Parent page identifier from the exporting chapter.
    pub page_id: String,

    /// Ordinal index of the page within its chapter.
    pub page_index: usize,

    /// Horizontal coordinate of the unit bounding box on the page.
    pub x_coord: f64,
    /// Vertical coordinate of the unit bounding box on the page.
    pub y_coord: f64,

    /// Whether this unit represents a speech bubble area.
    pub is_bubble: bool,

    /// Translated text content, or [`None`] if not translated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// Identifier of the translator user, or [`None`]. This is export metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translator_id: Option<String>,

    /// Whether this unit has been proofread.
    pub is_proofread: bool,
    /// Proofread text content, or [`None`] if not proofread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    /// Identifier of the proofreader user, or [`None`]. This is export metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofreader_id: Option<String>,
}
