use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::util::PatchField;

/// One normalized Unit mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitEdit {
    /// Create, restore, or patch one Unit.
    Save {
        //
        /// Permanent target Unit ID.
        id: String,
        /// Three-state successor pointer patch.
        next_id: PatchField<String>,

        /// Optional speech-bubble flag replacement.
        is_bubble: Option<bool>,
        /// Optional coordinate replacement.
        coord: Option<UnitCoord>,

        /// Three-state translation patch.
        translation: PatchField<UnitTranslation>,
        /// Three-state revision patch.
        revision: PatchField<UnitRevision>,
    },

    /// Hide one persisted Unit while retaining its chain node and content.
    Delete {
        /// Permanent target Unit ID.
        id: String,
    },
}
