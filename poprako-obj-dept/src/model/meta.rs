use crate::key::ObjKey;

/// Latest metadata for one business object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjMeta {
    //
    /// Current logical object key.
    pub key: ObjKey,
    /// Whether reads may expose the object.
    ///
    /// A client upload mark enables availability optimistically. The object
    /// actor may revoke it later when its presence check fails.
    pub is_avail: bool,
    /// Opaque content hash.
    pub hash: Vec<u8>,
    /// Validated object suffix.
    pub ext: String,
}
