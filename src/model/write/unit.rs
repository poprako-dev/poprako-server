use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::util::Patch;

/// One normalized Unit mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitEdit {
    //
    /// Creates one Unit with complete structural fields.
    Create {
        //
        /// Permanent Unit ID.
        id: String,
        /// Unit before which this Unit is inserted, or the tail.
        next_id: Option<String>,

        /// Whether this Unit identifies a speech bubble.
        is_bubble: bool,
        /// Initial page-relative coordinate.
        coord: UnitCoord,

        /// Optional initial translation.
        translation: Option<UnitTranslation>,
        /// Optional initial revision.
        revision: Option<UnitRevision>,
    },

    /// Restores or patches one persisted Unit.
    Save {
        //
        /// Permanent target Unit ID.
        id: String,
        /// Three-state successor pointer patch.
        next_id: Patch<String>,

        /// Optional speech-bubble flag replacement.
        is_bubble: Option<bool>,
        /// Optional coordinate replacement.
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
