use crate::key::ObjKey;
use crate::model::slot::ObjSlot;

/// Selected generation and whether it requires a direct upload.
#[derive(Clone, PartialEq, Eq)]
pub enum ObjAlloc {
    /// Identical content is already available at this generation.
    Avail {
        /// Exact key of the reused object.
        key: ObjKey,
    },
    /// New or pending content needs this signed upload capability.
    Upload {
        /// Signed upload capability bound to the selected object key.
        slot: ObjSlot,
    },
}

impl ObjAlloc {
    /// Returns the exact generation selected by this allocation.
    #[must_use]
    pub const fn key(&self) -> &ObjKey {
        //
        match self {
            //
            Self::Avail { key } => key,

            Self::Upload { slot } => &slot.key,
        }
    }

    /// Returns the upload capability when the selected generation needs bytes.
    #[must_use]
    pub fn into_slot(self) -> Option<ObjSlot> {
        //
        match self {
            //
            Self::Avail { .. } => None,

            Self::Upload { slot } => Some(slot),
        }
    }
}
