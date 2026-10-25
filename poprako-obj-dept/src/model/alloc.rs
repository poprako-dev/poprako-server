use crate::key::{ObjKey, ObjKind};
use crate::model::slot::ObjSlot;

/// Selected generation and whether it requires a direct upload.
#[derive(PartialEq, Eq)]
pub enum ObjAlloc<K>
where
    K: ObjKind,
{
    /// Identical content is already available at this generation.
    Avail {
        /// Exact key of the reused object.
        key: ObjKey<K>,
    },
    /// New or pending content needs this signed upload capability.
    Upload {
        /// Signed upload capability bound to the selected object key.
        slot: ObjSlot<K>,
    },
}

impl<K> Clone for ObjAlloc<K>
where
    K: ObjKind + Clone,
    K::Id: Clone,
{
    // Clones the value when its owned identity supports cloning.
    fn clone(&self) -> Self {
        //
        match self {
            //
            Self::Avail { key } => Self::Avail { key: key.clone() },

            Self::Upload { slot } => Self::Upload { slot: slot.clone() },
        }
    }
}

impl<K> ObjAlloc<K>
where
    K: ObjKind,
{
    /// Returns the exact generation selected by this allocation.
    #[must_use]
    pub const fn key(&self) -> &ObjKey<K> {
        //
        match self {
            //
            Self::Avail { key } => key,

            Self::Upload { slot } => &slot.key,
        }
    }

    /// Returns the upload capability when the selected generation needs bytes.
    #[must_use]
    pub fn into_slot(self) -> Option<ObjSlot<K>> {
        //
        match self {
            //
            Self::Avail { .. } => None,

            Self::Upload { slot } => Some(slot),
        }
    }
}
