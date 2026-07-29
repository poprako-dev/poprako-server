//! Domain models for page text units.

use time::OffsetDateTime;

/// A persisted page unit in final page order.
#[cfg_attr(test, derive(Clone))]
pub struct UnitInfo {
    //
    /// Server-assigned unique identifier for this unit.
    pub id: String,

    /// Foreign key referencing the page this unit belongs to.
    pub page_id: String,
    /// Display ordering index within the page, zero-based.
    pub index: i32,

    /// Whether this unit is a speech bubble contour.
    pub is_bubble: bool,
    /// Whether the proofread text has been reviewed and accepted.
    pub is_proofread: bool,

    /// Horizontal coordinate of the unit on the page image.
    pub x_coord: f64,
    /// Vertical coordinate of the unit on the page image.
    pub y_coord: f64,

    /// Translated text content, absent when not yet translated.
    pub translated_text: Option<String>,
    /// User who last modified the translated text.
    pub last_translator_id: Option<String>,

    /// Proofread (reviewed) text content, absent when not yet proofread.
    pub proofread_text: Option<String>,
    /// User who last modified the proofread text.
    pub last_proofreader_id: Option<String>,

    /// Timestamp when this unit was first inserted.
    pub created_at: OffsetDateTime,
    /// Timestamp when this unit was last modified.
    pub updated_at: OffsetDateTime,
}

impl UnitInfo {
    /// Reports whether this unit has meaningful translated content.
    pub fn is_translated(&self) -> bool {
        has_text(&self.translated_text) || has_text(&self.proofread_text)
    }
}

/// Complete mutable payload supplied by unit opers.
#[cfg_attr(test, derive(Clone))]
pub struct UnitContent {
    //
    /// Whether this unit is a speech bubble contour.
    pub is_bubble: bool,
    /// Whether the proofread text has been reviewed and accepted.
    pub is_proofread: bool,

    /// Horizontal coordinate of the unit on the page image.
    pub x_coord: f64,
    /// Vertical coordinate of the unit on the page image.
    pub y_coord: f64,

    /// Translated text content, absent when not yet translated.
    pub translated_text: Option<String>,
    /// User who last modified the translated text.
    pub last_translator_id: Option<String>,

    /// Proofread (reviewed) text content, absent when not yet proofread.
    pub proofread_text: Option<String>,
    /// User who last modified the proofread text.
    pub last_proofreader_id: Option<String>,
}

/// One compact unit difference submitted by a client.
///
/// `opers` are applied in order. Each create and save carries a `before_id`
/// that places it relative to the surviving server order; `None` (or a
/// `before_id` the server cannot find) appends the unit to the tail.
#[cfg_attr(test, derive(Clone))]
pub struct UnitDiff {
    //
    /// Foreign key of the page being diffed.
    pub page_id: String,
    /// Ordered list of unit operations to apply.
    pub opers: Vec<UnitOper>,
}

/// One ordered unit oper submitted by a client.
///
/// `Create` carries a client-provided identifier that is resolved before the
/// transaction starts. `Save` carries a server identifier and is applied as an
/// upsert so a concurrently deleted unit can be restored. `before_id` places a
/// created or saved unit relative to the surviving order; `None` or an absent
/// anchor appends it to the tail.
#[cfg_attr(test, derive(Clone))]
pub enum UnitOper {
    /// Create a new unit with a client id and content payload.
    Create {
        //
        /// Client-assigned local identifier for the new unit.
        id: String,
        /// Text content and translation state for the unit.
        payload: UnitContent,
        /// Optional unit ID to place this new unit before in ordering.
        before_id: Option<String>,
    },

    /// Update an existing unit with new content payload.
    Save {
        //
        /// Server-assigned identifier of the unit to update.
        id: String,
        /// Updated text content and translation state.
        payload: UnitContent,
        /// Optional unit ID to reorder this unit before.
        before_id: Option<String>,
    },

    /// Remove an existing unit by server id.
    Delete {
        /// Server-assigned identifier of the unit to remove.
        id: String,
    },
}

/// Persisted index for one surviving unit.
#[derive(Clone)]
pub struct UnitIndex {
    //
    /// Server-assigned identifier of the indexed unit.
    pub id: String,
    /// Zero-based display ordering index within the page.
    pub index: i32,
}

/// Index update for one unit whose persisted order changed.
#[cfg_attr(test, derive(Clone))]
pub struct UnitIndexUpdate {
    //
    /// Server-assigned identifier of the reordered unit.
    pub id: String,
    /// New zero-based display ordering index for this unit.
    pub index: i32,
}

/// Unit count snapshot for a page or counter delta target.
#[derive(Clone, Copy, Default)]
pub struct UnitCounters {
    //
    /// Total number of units on the page or target.
    pub total_unit_count: i32,
    /// Number of units with translated content.
    pub translated_unit_count: i32,
    /// Number of units with proofread content.
    pub proofread_unit_count: i32,
}

/// Delta between two unit counter snapshots.
#[derive(Default)]
#[cfg_attr(test, derive(Clone, Copy))]
pub struct UnitCounterDelta {
    //
    /// Change in total unit count since the reference snapshot.
    pub total_unit_count: i32,
    /// Change in translated unit count since the reference snapshot.
    pub translated_unit_count: i32,
    /// Change in proofread unit count since the reference snapshot.
    pub proofread_unit_count: i32,
}

/// Mapping from a client local unit id to a server unit id.
#[cfg_attr(test, derive(Clone))]
pub struct UnitIdMapper {
    //
    /// Client-provided local identifier before server resolution.
    pub local_id: String,
    /// Server-assigned identifier mapped from the local id.
    pub unit_id: String,
}

/// Result of applying unit opers to a page snapshot.
#[cfg_attr(test, derive(Clone))]
pub struct UnitApplyAck {
    //
    /// Final set of applied unit operations.
    pub opers: Vec<UnitOper>,
    /// Mappings from client local ids to resolved server ids.
    pub local_id_map: Vec<UnitIdMapper>,
}

/// Returns `true` if the optional string is present and non-empty.
fn has_text(text: &Option<String>) -> bool {
    text.as_ref()
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}
