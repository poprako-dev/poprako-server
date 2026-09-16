//! Durable Unit save receipt.
use crate::model::shared::unit_save::UnitSaveCreatedUnitId;

/// New receipt committed with the Unit edit batch.
#[derive(Clone)]
pub struct UnitSaveEntry {
    /// Authenticated actor owning the save identity.
    pub user_id: String,
    /// Page receiving the edits.
    pub page_id: String,
    /// Client-generated UUID identifying this immutable batch.
    pub save_id: String,

    /// SHA-256 of the canonical typed edit payload.
    pub payload_digest: Vec<u8>,
    /// Explicit local-to-permanent identity pairs.
    pub created_unit_ids: Vec<UnitSaveCreatedUnitId>,
}
