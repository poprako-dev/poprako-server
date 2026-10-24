//! Bounded Unit save tokens shared by every clone of a cache adapter.

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
// PostgreSQL commit and replay regression tests.
mod rdb_tests;

#[cfg(test)]
// Cache coordination and eviction contract tests.
mod tests;

use std::array::from_fn;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher as _;
use std::mem::size_of;
use std::sync::Arc;

use moka::sync::SegmentedCache;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

use crate::config::cache::CacheConfig;
use crate::part::cache::unit_save::{
    UnitSaveCache as UnitSavePort, UnitSaveGuard,
};
use crate::part_impl::cache::mem_impl::MemCache;
use crate::value::unit_save::UnitSaveCreatedUnitId;

// Covers UUID, Arc, allocation headers, and amortized Moka metadata.
const ENTRY_OVERHEAD: usize = 320;

// Charges string capacities, pair storage, and two allocation headers per pair.
fn entry_weight(ids: &[UnitSaveCreatedUnitId], min_entry_weight: u64) -> u32 {
    //
    let bytes = ids.iter().fold(ENTRY_OVERHEAD, |total, pair| {
        //
        total
            .saturating_add(size_of::<UnitSaveCreatedUnitId>())
            .saturating_add(pair.local_id.capacity())
            .saturating_add(pair.unit_id.capacity())
            .saturating_add(32)
    });

    let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);

    u32::try_from(bytes.max(min_entry_weight)).unwrap_or(u32::MAX)
}

// Locks are separate from evictable entries and stay bounded under load.
const LOCK_SHARDS: usize = 256;

/// Borrows the shared cache while owning exclusion for this save key.
pub struct UnitSaveCacheGuard<'a> {
    /// Successful token storage.
    cache: &'a SegmentedCache<Uuid, Arc<[UnitSaveCreatedUnitId]>>,
    /// Identity protected by this guard.
    save_id: Uuid,
    /// Held through transaction completion and synchronous cache publication.
    _guard: MutexGuard<'a, ()>,
}

impl UnitSaveGuard for UnitSaveCacheGuard<'_> {
    // Read the immutable mapping shared by completed retries.
    fn created_unit_ids(&self) -> Option<Arc<[UnitSaveCreatedUnitId]>> {
        self.cache.get(&self.save_id)
    }

    // Publish a completed mapping under the held save-key lock.
    fn record(&self, created_unit_ids: Arc<[UnitSaveCreatedUnitId]>) {
        self.cache.insert(self.save_id, created_unit_ids);
    }
}

/// Shared segmented `TinyLFU` cache with bounded asynchronous coordination.
#[derive(Clone)]
pub struct UnitSaveCache {
    /// Successful tokens, bounded independently of in-flight requests.
    tokens: SegmentedCache<Uuid, Arc<[UnitSaveCreatedUnitId]>>,

    /// Fixed lock stripes span lookup, transaction completion, and publication.
    locks: Arc<[Mutex<()>; LOCK_SHARDS]>,
    /// Randomized stripe assignment for client-generated identities.
    hasher: RandomState,
}

impl UnitSaveCache {
    /// Builds an empty process-local cache with the requested estimated-byte budget.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        //
        let max_capacity = config.unit_save_limit_bytes();

        let min_entry_weight = config.unit_save_min_entry_weight();

        Self {
            tokens: SegmentedCache::builder(16)
                .max_capacity(max_capacity)
                .time_to_live(config.unit_save_ttl)
                .weigher(move |_, ids: &Arc<[UnitSaveCreatedUnitId]>| {
                    entry_weight(ids, min_entry_weight)
                })
                .build(),
            locks: Arc::new(from_fn(|_| Mutex::new(()))),
            hasher: RandomState::new(),
        }
    }

    /// Acquires a stable lock that cannot disappear when a token is evicted.
    pub async fn lock_save(&self, save_id: Uuid) -> UnitSaveCacheGuard<'_> {
        //
        let [stripe_byte, ..] = self.hasher.hash_one(save_id).to_le_bytes();

        let stripe = usize::from(stripe_byte);

        let [first_lock, ..] = self.locks.as_ref();

        let lock = self.locks.get(stripe).unwrap_or(first_lock);

        let guard = lock.lock().await;

        UnitSaveCacheGuard {
            cache: &self.tokens,
            save_id,
            _guard: guard,
        }
    }

    /// Drains cache maintenance before counting tokens in test assertions.
    #[cfg(test)]
    #[must_use]
    pub fn token_count(&self) -> u64 {
        //
        self.tokens.run_pending_tasks();

        self.tokens.entry_count()
    }
}

impl Default for UnitSaveCache {
    // Use the production estimated-memory budget.
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

impl UnitSavePort for MemCache {
    // Keep the cache guard borrowed from this cache adapter handle.
    type Guard<'a> = UnitSaveCacheGuard<'a>;

    // Share the production cache and its exclusion guard.
    fn lock_save(
        &self,
        save_id: Uuid,
    ) -> impl Future<Output = Self::Guard<'_>> + Send {
        self.unit_saves.lock_save(save_id)
    }
}
