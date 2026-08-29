use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

/// Owned logical identity of one object generation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjKey {
    /// Stable business-object identifier.
    pub id: String,
    /// Generation allocated for this object identity.
    pub version: u32,
}

impl ObjKey {
    /// Borrows this logical key.
    #[must_use]
    pub fn as_ref(&self) -> ObjKeyRef<'_> {
        //
        ObjKeyRef {
            id: &self.id,
            version: self.version,
        }
    }

    /// Encodes the physical key inside one immutable object namespace.
    #[must_use]
    pub fn encode(&self, namespace: &str) -> String {
        self.as_ref().encode(namespace)
    }
}

/// Borrowed logical identity of one object generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjKeyRef<'a> {
    /// Stable business-object identifier.
    pub id: &'a str,
    /// Generation allocated for this object identity.
    pub version: u32,
}

impl ObjKeyRef<'_> {
    /// Encodes the physical key inside one immutable object namespace.
    #[must_use]
    pub fn encode(self, namespace: &str) -> String {
        //
        let id = URL_SAFE_NO_PAD.encode(self.id.as_bytes());

        format!("{}/{}/{}", namespace, id, self.version)
    }
}
