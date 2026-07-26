use time::OffsetDateTime;

use crate::model::shared::unit::UnitCoord;

pub struct UnitOrder {
    pub id: String,
    pub next_id: Option<String>,

    pub is_hidden: bool,
}

/// A persisted page unit in final page order.
#[cfg_attr(test, derive(Clone))]
pub struct UnitInfo {
    //
    /// Server-assigned unique identifier for this unit.
    pub id: String,

    /// Foreign key referencing the page this unit belongs to.
    ///
    /// NOTE: the order(or index) should be revealed by the
    /// order of array in which this unit is stored.
    pub page_id: String,
    /// Whether this unit is a speech bubble contour.
    pub is_bubble: bool,

    pub coord: UnitCoord,

    /// Translated text content, absent when not yet translated.
    pub translated_text: Option<String>,
    /// User who last modified the translated text.
    pub last_translator_id: Option<String>,

    /// Whether the proofread text has been reviewed and accepted.
    pub is_proofread: bool,
    /// Proofread (reviewed) text content, absent when not yet proofread.
    pub proofread_text: Option<String>,
    /// User who last modified the proofread text.
    pub last_proofreader_id: Option<String>,

    pub hidden_at: Option<OffsetDateTime>,

    /// Timestamp when this unit was first inserted.
    pub created_at: OffsetDateTime,
    /// Timestamp when this unit was last modified.
    pub updated_at: OffsetDateTime,
}
