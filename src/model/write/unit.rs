use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::util::PatchField;

pub enum UnitEdit {
    /// Upsert a unit with given content payload.
    Save {
        id: String,
        /// Optional unit ID to place this new unit before in ordering.
        next_id: PatchField<String>,

        /// Whether this unit is a speech bubble contour.
        is_bubble: bool,
        /// Whether the proofread text has been reviewed and accepted.
        is_proofread: bool,

        coord: UnitCoord,

        translation: PatchField<UnitTranslation>,
        revision: PatchField<UnitRevision>,
    },

    /// Remove an existing unit by server id.
    Delete {
        /// Server-assigned identifier of the unit to **hide**.
        id: String,
    },
}
