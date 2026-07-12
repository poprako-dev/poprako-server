//! Domain models for page text units.

use time::OffsetDateTime;

/// A persisted page unit in final page order.
#[cfg_attr(test, derive(Clone))]
pub struct Info {
    pub id: String,

    pub page_id: String,
    pub index: i32,

    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    pub translated_text: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub last_proofreader_id: Option<String>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Info {
    /// Reports whether this unit has meaningful translated content.
    pub fn is_translated(&self) -> bool {
        has_text(&self.translated_text) || has_text(&self.proofread_text)
    }
}

/// Complete mutable payload supplied by unit opers.
#[cfg_attr(test, derive(Clone))]
pub struct Payload {
    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    pub translated_text: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub last_proofreader_id: Option<String>,
}

/// One compact unit difference submitted by a client.
///
/// `opers` are applied in order. Each create and save carries a `before_id`
/// that places it relative to the surviving server order; `None` (or a
/// `before_id` the server cannot find) appends the unit to the tail.
#[cfg_attr(test, derive(Clone))]
pub struct Diff {
    pub page_id: String,
    pub opers: Vec<Oper>,
}

/// One ordered unit oper submitted by a client.
///
/// `Create` carries a client-provided identifier that is resolved before the
/// transaction starts. `Save` carries a server identifier and is applied as an
/// upsert so a concurrently deleted unit can be restored. `before_id` places a
/// created or saved unit relative to the surviving order; `None` or an absent
/// anchor appends it to the tail.
#[cfg_attr(test, derive(Clone))]
pub enum Oper {
    Create {
        id: String,
        payload: Payload,
        before_id: Option<String>,
    },
    Save {
        id: String,
        payload: Payload,
        before_id: Option<String>,
    },
    Delete {
        id: String,
    },
}

/// Persisted index for one surviving unit.
#[derive(Clone)]
pub struct Index {
    pub id: String,
    pub index: i32,
}

/// Index update for one unit whose persisted order changed.
#[cfg_attr(test, derive(Clone))]
pub struct IndexUpdate {
    pub id: String,
    pub index: i32,
}

/// Unit count snapshot for a page or counter delta target.
#[derive(Clone, Copy, Default)]
pub struct Counters {
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Delta between two unit counter snapshots.
#[derive(Default)]
#[cfg_attr(test, derive(Clone, Copy))]
pub struct CounterDelta {
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Mapping from a client local unit id to a server unit id.
#[cfg_attr(test, derive(Clone))]
pub struct IdMapper {
    pub local_id: String,
    pub unit_id: String,
}

/// Result of applying unit opers to a page snapshot.
#[cfg_attr(test, derive(Clone))]
pub struct ApplyAck {
    pub opers: Vec<Oper>,
    pub local_id_map: Vec<IdMapper>,
}

/// Returns `true` if the optional string is present and non-empty.
fn has_text(text: &Option<String>) -> bool {
    text.as_ref()
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}
