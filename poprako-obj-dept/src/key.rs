use serde::{Deserialize, Serialize};

use crate::rest::ObjDeptRest;

/// Maps one typed business identity to its complete physical object key.
pub trait KeyMap {
    /// Business identity required by this object kind.
    type Dom;

    /// Complete physical key understood by the storage adapter.
    type Img;

    /// Returns the stable object-table identity.
    fn id(value: &Self::Dom) -> &str;

    /// Returns the suffix persisted with active object metadata.
    fn ext(value: &Self::Dom) -> &str;

    /// Builds the complete physical key for one allocated generation.
    fn forward(value: &Self::Dom, version: u32) -> Self::Img;

    /// Decodes one complete physical key into its business identity and generation.
    ///
    /// # Errors
    ///
    /// Returns an error when the physical key is outside this object's contract.
    fn reverse(value: &Self::Img) -> ObjDeptRest<(Self::Dom, u32)>;
}

/// Persisted identity of one exact object generation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjKey {
    //
    /// Stable object-table identifier.
    pub id: String,

    /// Generation allocated for this object identity.
    pub version: u32,

    /// Complete immutable physical object key.
    pub image: String,
}

/// Client-addressable logical object generation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjGeneration {
    //
    /// Stable object-table identifier.
    pub id: String,

    /// Generation allocated for this object identity.
    pub version: u32,
}
