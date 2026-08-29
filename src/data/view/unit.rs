//! View DTOs for the unit domain.

//! Data transfer objects for page Unit use cases.
//!
//! Types in this module describe how client-supplied edit payloads are
//! represented and how persisted Unit rows are projected back into API-facing
//! response types.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::unit::UnitInfo;

/// Presentation-ready visible Unit information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitInfoView {
    /// Permanent Unit ID.
    pub id: String,
    /// Owning Page ID.
    pub page_id: String,

    /// Whether the Unit identifies a speech bubble.
    pub is_bubble: bool,
    /// Whether the current revision is approved.
    pub is_proofread: bool,

    /// Horizontal page-relative coordinate.
    pub x_coord: f64,
    /// Vertical page-relative coordinate.
    pub y_coord: f64,

    /// Current translated text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// ID of the translator who last assigned translation content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_translator_id: Option<String>,

    /// Current proofread text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    /// ID of the proofreader who last assigned revision content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_proofreader_id: Option<String>,

    /// Creation time as Unix milliseconds.
    pub created_at: i64,
    /// Last update time as Unix milliseconds.
    pub updated_at: i64,
}

impl From<UnitInfo> for UnitInfoView {
    // Map persisted unit info model into API value shape.
    fn from(model: UnitInfo) -> Self {
        //
        Self {
            id: model.id,
            page_id: model.page_id,
            is_bubble: model.is_bubble,
            is_proofread: model.is_proofread,
            x_coord: model.coord.x_coord,
            y_coord: model.coord.y_coord,
            translated_text: model.translated_text,
            last_translator_id: model.last_translator_id,
            proofread_text: model.proofread_text,
            last_proofreader_id: model.last_proofreader_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}
