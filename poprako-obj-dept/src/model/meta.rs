use crate::key::ObjKey;

/// Latest metadata for one business object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjMeta {
    //
    /// Current logical object key.
    pub key: ObjKey,
    /// Whether remote storage has been verified.
    pub f_is_uploaded: bool,
    /// Opaque content hash.
    pub hash: Vec<u8>,
    /// Validated object suffix.
    pub ext: String,
}
