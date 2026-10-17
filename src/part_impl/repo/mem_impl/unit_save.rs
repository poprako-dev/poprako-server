//! Bounded Unit save tokens shared by every clone of a repository.

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

use crate::part::repo::unit_save::{UnitSaveGuard, UnitSaveRepo};
use crate::part_impl::repo::HybRepo;
use crate::value::unit_save::UnitSaveCreatedUnitId;

// Weight estimates retained memory, including an allowance for cache metadata.
const MAX_CACHE_WEIGHT: u64 = 256 * 1024 * 1024;

// Maximum retained save identities under the weighted budget.
const MAX_TOKENS: u64 = 500_000;

// The minimum charge also limits entry count under the same weighted policy.
const MIN_ENTRY_WEIGHT: u64 = MAX_CACHE_WEIGHT.div_ceil(MAX_TOKENS);

// Covers UUID, Arc, allocation headers, and amortized Moka metadata.
const ENTRY_OVERHEAD: usize = 320;

// Charges string capacities, pair storage, and two allocation headers per pair.
fn entry_weight(ids: &[UnitSaveCreatedUnitId]) -> u32 {
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

    u32::try_from(bytes.max(MIN_ENTRY_WEIGHT)).unwrap_or(u32::MAX)
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
    pub fn new(max_capacity: u64) -> Self {
        Self {
            tokens: SegmentedCache::builder(16)
                .max_capacity(max_capacity)
                .weigher(|_, ids: &Arc<[UnitSaveCreatedUnitId]>| {
                    entry_weight(ids)
                })
                .build(),
            locks: Arc::new(from_fn(|_| Mutex::new(()))),
            hasher: RandomState::new(),
        }
    }

    /// Acquires a stable lock that cannot disappear when a token is evicted.
    #[allow(
        clippy::indexing_slicing,
        reason = "A hash byte is always within the 256-element lock array."
    )]
    pub async fn lock_save(&self, save_id: Uuid) -> UnitSaveCacheGuard<'_> {
        //
        let stripe =
            usize::from(self.hasher.hash_one(save_id).to_le_bytes()[0]);

        let guard = self.locks[stripe].lock().await;

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
        Self::new(MAX_CACHE_WEIGHT)
    }
}

impl UnitSaveRepo for HybRepo {
    // Keep the cache guard borrowed from this repository handle.
    type Guard<'a> = UnitSaveCacheGuard<'a>;

    // Share the production cache and its exclusion guard.
    fn lock_save(
        &self,
        save_id: Uuid,
    ) -> impl Future<Output = Self::Guard<'_>> + Send {
        self.unit_saves.lock_save(save_id)
    }
}
