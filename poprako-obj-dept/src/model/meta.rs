use crate::key::{ObjId as _, ObjKey, ObjKind};
use crate::model::record::ObjMetaRecord;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Latest metadata for one business object.
#[derive(PartialEq, Eq)]
pub struct ObjMeta<K>
where
    K: ObjKind,
{
    /// Current logical object key.
    pub key: ObjKey<K>,
    /// Whether reads may expose the object.
    ///
    /// A client upload mark enables availability optimistically. The object
    /// actor may revoke it later when its presence check fails.
    pub is_avail: bool,
    /// Opaque content hash.
    pub hash: Vec<u8>,
    /// Validated object suffix.
    pub ext: K::Ext,
}

impl<K> Clone for ObjMeta<K>
where
    K: ObjKind + Clone,
    K::Id: Clone,
    K::Ext: Clone,
{
    // Clones the value when its owned identity supports cloning.
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            is_avail: self.is_avail,
            hash: self.hash.clone(),
            ext: self.ext.clone(),
        }
    }
}

impl ObjMetaRecord {
    /// Restores metadata at the concrete object-kind persistence seam.
    ///
    /// # Errors
    /// Returns an error for invalid persisted identity, suffix, or location.
    pub fn decode<K>(self) -> ObjDeptRest<ObjMeta<K>>
    where
        K: ObjKind,
    {
        //
        let ext = K::Ext::from_owned(self.ext).map_err(|_| {
            ObjDeptError::Unrecoverable {
                message: "invalid persisted object suffix".into(),
            }
        })?;

        let key = ObjKey::<K>::from_record(self.key)?;

        if key.ext() != ext.as_str() {
            //
            return Err(ObjDeptError::Unrecoverable {
                message: "object suffix differs from construction data".into(),
            });
        }

        Ok(ObjMeta {
            key,
            is_avail: self.is_avail,
            hash: self.hash,
            ext,
        })
    }
}
