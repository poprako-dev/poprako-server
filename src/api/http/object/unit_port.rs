//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::result::Wire;
use crate::data::view::unit_port::UnitTranslationPortView;

/// Unit object exchanged by the `PopRaKo` translation port.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitTranslationPortObj {
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
    /// Whether this Unit is flagged for later review.
    #[serde(default)]
    pub is_flagged: bool,

    /// Translation text annotation delivered to typesetters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typography_note: Option<String>,
    /// Communication between translators and proofreaders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_note: Option<String>,

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

impl From<UnitTranslationPortView> for UnitTranslationPortObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: UnitTranslationPortView) -> Self {
        Self {
            unit_id: value.unit_id,
            unit_index: value.unit_index,
            page_id: value.page_id,
            page_index: value.page_index,
            x_coord: value.x_coord,
            y_coord: value.y_coord,
            is_bubble: value.is_bubble,
            is_flagged: value.is_flagged,
            typography_note: value.typography_note,
            translation_note: value.translation_note,
            translated_text: value.translated_text,
            translator_id: value.translator_id,
            is_proofread: value.is_proofread,
            proofread_text: value.proofread_text,
            proofreader_id: value.proofreader_id,
        }
    }
}

impl Wire for UnitTranslationPortView {
    // Serializable wire payload produced by this conversion.
    type Target = UnitTranslationPortObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
