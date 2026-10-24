//! Process-local coordination and retention of Unit save tokens.

use std::sync::Arc;

use uuid::Uuid;

use crate::value::unit_save::UnitSaveCreatedUnitId;

/// Exclusive access to a save identity, independent of token eviction.
pub trait UnitSaveGuard {
    /// Reads the committed mapping after the caller checked authorization.
    fn created_unit_ids(&self) -> Option<Arc<[UnitSaveCreatedUnitId]>>;

    /// Records completion only after the database transaction commits.
    fn record(&self, created_unit_ids: Arc<[UnitSaveCreatedUnitId]>);
}

/// Coordinates retries across the full transaction and token publication.
pub trait UnitSaveCache {
    /// Adapter-owned exclusion guard borrowing this cache adapter.
    type Guard<'a>: UnitSaveGuard
    where
        Self: 'a;

    /// Holds process-local exclusion until the returned guard is dropped.
    /// A dropped or failed transaction must not publish a successful token.
    fn lock_save(
        &self,
        save_id: Uuid,
    ) -> impl Future<Output = Self::Guard<'_>> + Send;
}
