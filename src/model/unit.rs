//! Domain models for page text units.

use time::OffsetDateTime;

/// A persisted page unit in final page order.
#[derive(Clone)]
pub struct UnitInfo {
    pub id: String,

    pub page_id: String,
    pub index: i32,

    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    pub translated_text: Option<String>,
    pub translator_comment: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub proofreader_comment: Option<String>,
    pub last_proofreader_id: Option<String>,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl UnitInfo {
    /// Reports whether this unit has meaningful translated content.
    pub fn is_translated(&self) -> bool {
        has_text(&self.translated_text) || has_text(&self.proofread_text)
    }
}

/// Full server-side unit snapshot supplied by a write operation.
#[derive(Clone)]
pub struct UnitServerSnapshot {
    pub id: String,

    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    pub translated_text: Option<String>,
    pub translator_comment: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub proofreader_comment: Option<String>,
    pub last_proofreader_id: Option<String>,
}

/// Full local unit snapshot supplied by an insert operation.
#[derive(Clone)]
pub struct UnitLocalSnapshot {
    pub local_id: String,

    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    pub translated_text: Option<String>,
    pub translator_comment: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub proofreader_comment: Option<String>,
    pub last_proofreader_id: Option<String>,
}

/// One ordered unit operation submitted by a client.
#[cfg_attr(test, derive(Clone))]
pub enum UnitOper {
    Update {
        unit: UnitServerSnapshot,
    },
    MoveBefore {
        unit: UnitServerSnapshot,
        before_id: Option<String>,
    },
    InsertBefore {
        unit: UnitLocalSnapshot,
        before_id: Option<String>,
    },
    Delete {
        unit_id: String,
    },
}

/// Unit count snapshot for a page or counter delta target.
#[derive(Clone, Copy, Default)]
pub struct UnitCounters {
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Delta between two unit counter snapshots.
#[derive(Clone, Copy, Default)]
pub struct UnitCounterDelta {
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Mapping from a client local unit id to a server unit id.
#[cfg_attr(test, derive(Clone))]
pub struct UnitIdMapper {
    pub local_id: String,
    pub unit_id: String,
}

/// Result of applying unit operations to a page snapshot.
#[cfg_attr(test, derive(Clone))]
pub struct UnitApplyAck {
    pub unit_infos: Vec<UnitInfo>,
    pub id_mapper: Vec<UnitIdMapper>,
    pub counters: UnitCounters,
}

fn has_text(text: &Option<String>) -> bool {
    text.as_ref()
        .map(|value| !value.is_empty())
        .unwrap_or(false)
}
