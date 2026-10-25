//! Unit read projections.

use time::OffsetDateTime;

use crate::model::shared::unit::UnitCoord;
use crate::result::BaseRest;

/// One Unit node in the complete persisted page chain.
#[derive(Clone, PartialEq, Eq)]
pub struct UnitOrder {
    /// Permanent Unit ID.
    pub id: String,
    /// Permanent ID of the following Unit.
    pub next_id: Option<String>,

    /// Whether this Unit is a tombstone.
    pub is_hidden: bool,
}

/// A persisted page Unit.
#[derive(Clone)]
pub struct UnitInfo {
    /// Permanent Unit ID.
    pub id: String,

    /// Owning source page ID.
    pub page_source_id: String,
    /// Permanent ID of the following Unit.
    pub next_id: Option<String>,

    /// Whether this Unit identifies a speech bubble.
    pub is_bubble: bool,
    /// Whether this Unit is flagged for later review.
    pub is_flagged: bool,

    /// source page-relative coordinate.
    pub coord: UnitCoord,

    /// Translation text annotation delivered to typesetters.
    pub typography_note: Option<String>,
    /// Communication between translators and proofreaders.
    pub translation_note: Option<String>,

    /// Current translated text.
    pub translated_text: Option<String>,
    /// ID of the translator who last assigned translation content.
    pub last_translator_id: Option<String>,

    /// Whether the current revision is approved.
    pub is_proofread: bool,
    /// Current proofread text.
    pub proofread_text: Option<String>,
    /// ID of the proofreader who last assigned revision content.
    pub last_proofreader_id: Option<String>,

    /// Tombstone creation time, or none while visible.
    pub hidden_at: Option<OffsetDateTime>,

    /// Creation time.
    pub created_at: OffsetDateTime,
    /// Last update time.
    pub updated_at: OffsetDateTime,
}

impl UnitInfo {
    /// Reports whether this Unit has usable translation, revision, or typesetting annotation.
    pub fn is_translated(&self) -> bool {
        //
        has_unit_text(self.translated_text.as_deref())
            || has_unit_text(self.proofread_text.as_deref())
            || has_unit_text(self.typography_note.as_deref())
    }
}

/// Reports whether an optional Unit text contains non-whitespace content.
pub fn has_unit_text(text: Option<&str>) -> bool {
    text.is_some_and(|value| !value.trim().is_empty())
}

/// Unit counters stored on a source page and its Chapter.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct UnitCountMetrics {
    /// Number of visible Units.
    pub total: usize,
    /// Number of visible translated Units.
    pub translated: usize,
    /// Number of visible proofread Units.
    pub proofread: usize,
}

impl UnitCountMetrics {
    /// Calculates the counter delta from this snapshot to the next snapshot.
    pub fn calc_delta(self, next: Self) -> BaseRest<UnitCountDelta> {
        //
        Ok(UnitCountDelta {
            total: signed_count(next.total, "total")?
                - signed_count(self.total, "total")?,
            translated: signed_count(next.translated, "translated")?
                - signed_count(self.translated, "translated")?,
            proofread: signed_count(next.proofread, "proofread")?
                - signed_count(self.proofread, "proofread")?,
        })
    }
}

/// Counter change applied to a Chapter after one source page mutation.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct UnitCountDelta {
    /// Visible Unit count change.
    pub total: i32,
    /// Visible translated Unit count change.
    pub translated: i32,
    /// Visible proofread Unit count change.
    pub proofread: i32,
}

// Convert a unit count into the signed representation used by counter deltas.
fn signed_count(value: usize, field: &str) -> BaseRest<i32> {
    //
    i32::try_from(value).map_err(|_| {
        //
        unrecoverable_error! {
            message: format!("unit count {} exceeds signed delta range", field);
            field,
            value,
            "unit count exceeds signed delta range",
        }
    })
}
