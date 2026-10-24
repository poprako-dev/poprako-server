//! Borrowed unit write payloads.
//! Owners remain alive through persistence; existing data is not copied.

use std::collections::BTreeMap;

use crate::model::write::unit::UnitEdit;
use crate::util::Patch;

/// Sparse mutation of one persisted Unit after final sequence planning.
#[derive(Default)]
pub struct UnitWriteChangeShape<'a> {
    /// Final successor assignment, clearing, or preservation.
    pub next_id: Patch<&'a str>,
    /// Visibility transition; None preserves the stored timestamp.
    pub is_hidden: Option<bool>,
    /// Submitted content fields, excluding order and visibility.
    pub edit: Option<&'a UnitEdit>,
}

impl UnitWriteChangeShape<'_> {
    /// Whether no structural or submitted content field needs writing.
    pub const fn is_empty(&self) -> bool {
        //
        self.next_id.is_skip()
            && self.is_hidden.is_none()
            && self.edit.is_none()
    }
}

/// Final inserts and sparse mutations for one normalized Unit save batch.
pub struct UnitWritePlanShape<'a> {
    /// Create edits containing final content and final successor pointers.
    pub creates: Vec<UnitEdit>,
    /// Existing rows with at least one field to write, keyed by Unit ID.
    pub changes: BTreeMap<&'a str, UnitWriteChangeShape<'a>>,
    /// Submitted existing targets omitted from writes but requiring validation.
    pub skipped_ids: Vec<&'a str>,
}
