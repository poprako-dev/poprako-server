//! Unit read projections.

use time::OffsetDateTime;

use crate::model::shared::unit::UnitCoord;

/// One Unit node in the complete persisted page chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitOrder {
    //
    /// Permanent Unit ID.
    pub id: String,
    /// Permanent ID of the following Unit.
    pub next_id: Option<String>,

    /// Whether this Unit is a tombstone.
    pub is_hidden: bool,
}

/// A persisted page Unit.
#[derive(Debug, Clone)]
pub struct UnitInfo {
    //
    /// Permanent Unit ID.
    pub id: String,

    /// Owning Page ID.
    pub page_id: String,
    /// Permanent ID of the following Unit.
    pub next_id: Option<String>,

    /// Whether this Unit identifies a speech bubble.
    pub is_bubble: bool,

    /// Page-relative coordinate.
    pub coord: UnitCoord,

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
    /// Reports whether this Unit has usable translation or revision text.
    pub fn is_translated(&self) -> bool {
        has_text(&self.translated_text) || has_text(&self.proofread_text)
    }
}

/// Unit counters stored on a Page and its Chapter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnitCounters {
    //
    /// Number of visible Units.
    pub total_unit_count: i32,
    /// Number of visible translated Units.
    pub translated_unit_count: i32,
    /// Number of visible proofread Units.
    pub proofread_unit_count: i32,
}

impl UnitCounters {
    /// Calculates the counter delta from this snapshot to the next snapshot.
    pub fn calc_delta(self, next: Self) -> UnitCounterDelta {
        //
        UnitCounterDelta {
            total_unit_count: next.total_unit_count - self.total_unit_count,
            translated_unit_count: next.translated_unit_count
                - self.translated_unit_count,
            proofread_unit_count: next.proofread_unit_count
                - self.proofread_unit_count,
        }
    }
}

/// Counter change applied to a Chapter after one Page mutation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnitCounterDelta {
    //
    /// Visible Unit count change.
    pub total_unit_count: i32,
    /// Visible translated Unit count change.
    pub translated_unit_count: i32,
    /// Visible proofread Unit count change.
    pub proofread_unit_count: i32,
}

// Reports whether a text field contains non-whitespace content.
fn has_text(text: &Option<String>) -> bool {
    // Ignore purely-whitespace values so counters only count usable content.
    text.as_ref().is_some_and(|value| !value.trim().is_empty())
}
