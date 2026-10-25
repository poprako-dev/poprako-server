//! Raw persistence values, separate from typed object operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Persisted snapshot with no compile-time object kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjKeyRecord {
    /// Stable business identifier.
    pub id: String,

    /// Exact generation.
    pub ver: u32,
    /// Complete construction data for the selected object kind.
    pub domain: Value,
}

/// Persisted active metadata used by storage adapters and fixtures.
#[derive(Clone, PartialEq, Eq)]
pub struct ObjMetaRecord {
    /// Exact physical and logical identity.
    pub key: ObjKeyRecord,

    /// Optimistic upload evidence.
    pub is_avail: bool,
    /// Content identity.
    pub hash: Vec<u8>,
    /// Stored suffix.
    pub ext: String,
}
