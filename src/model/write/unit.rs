use std::collections::BTreeMap;

use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::util::Patch;

/// One literal text transform applied against an original Unit field value.
#[derive(Clone, PartialEq, Eq)]
pub struct UnitTextTransform {
    /// Literal text selected in the original value.
    pub origin: String,
    /// Literal text inserted in place of each match.
    pub target: String,
}

/// Text transforms requested for one permanent Unit.
#[derive(Clone, PartialEq, Eq)]
pub struct UnitTransform {
    /// Permanent target Unit ID.
    pub unit_id: String,
    /// Literal transforms evaluated against the same original text.
    pub transforms: Vec<UnitTextTransform>,
}

/// One normalized Unit mutation.
#[derive(Clone, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum UnitEdit {
    /// Creates one Unit with complete structural fields.
    Create {
        /// Permanent Unit ID.
        id: String,
        /// Unit before which this Unit is inserted, or the tail.
        next_id: Option<String>,

        /// Whether this Unit identifies a speech bubble.
        is_bubble: bool,
        /// Whether this Unit is flagged for later review.
        is_flagged: bool,
        /// Initial page-relative coordinate.
        coord: UnitCoord,

        /// Optional initial translation.
        translation: Option<UnitTranslation>,
        /// Optional initial revision.
        revision: Option<UnitRevision>,
    },

    /// Restores or patches one persisted Unit.
    Save {
        /// Permanent target Unit ID.
        id: String,
        /// Three-state successor pointer patch.
        next_id: Patch<String>,

        /// Optional updated speech-bubble flag.
        is_bubble: Option<bool>,
        /// Optional review flag replacement.
        is_flagged: Option<bool>,
        /// Optional updated coordinate.
        coord: Option<UnitCoord>,

        /// Three-state translation patch.
        translation: Patch<UnitTranslation>,
        /// Three-state revision patch.
        revision: Patch<UnitRevision>,
    },

    /// Hide one persisted Unit while retaining its chain node and content.
    Delete {
        /// Permanent target Unit ID.
        id: String,
    },
}

/// Sparse mutation of one persisted Unit after final sequence planning.
#[derive(Default)]
pub struct UnitWriteChange<'a> {
    /// Final successor assignment, clearing, or preservation.
    pub next_id: Patch<&'a str>,
    /// Visibility transition; None preserves the stored timestamp.
    pub is_hidden: Option<bool>,
    /// Submitted content fields, excluding order and visibility.
    pub edit: Option<&'a UnitEdit>,
}

impl UnitWriteChange<'_> {
    /// Whether no structural or submitted content field needs writing.
    pub const fn is_empty(&self) -> bool {
        //
        self.next_id.is_skip()
            && self.is_hidden.is_none()
            && self.edit.is_none()
    }
}

/// Final inserts and sparse mutations for one normalized Unit save batch.
pub struct UnitWritePlan<'a> {
    /// Create edits containing final content and final successor pointers.
    pub creates: Vec<UnitEdit>,
    /// Existing rows with at least one field to write, keyed by Unit ID.
    pub changes: BTreeMap<&'a str, UnitWriteChange<'a>>,
    /// Submitted existing targets omitted from writes but requiring validation.
    pub skipped_ids: Vec<&'a str>,
}
