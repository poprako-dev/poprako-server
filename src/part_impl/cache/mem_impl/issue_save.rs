//! Bounded annotation save tokens with fixed lock stripes.

use std::array::from_fn;
use std::collections::hash_map::RandomState;
use std::hash::BuildHasher as _;
use std::sync::Arc;

use moka::sync::Cache;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

use crate::config::cache::CacheConfig;
use crate::part::cache::issue_save::{
    IssueSaveCache as IssueSavePort, IssueSaveGuard,
};
use crate::part_impl::cache::mem_impl::MemCache;
use crate::value::issue_save::IssueSaveReceipt;

/// Receipt protected independently of cache eviction.
pub struct IssueSaveCacheGuard<'a> {
    /// Shared completed receipt cache.
    cache: &'a Cache<Uuid, IssueSaveReceipt>,
    /// Immutable batch identity.
    save_id: Uuid,
    /// Held lock stripe.
    _guard: MutexGuard<'a, ()>,
}

impl IssueSaveGuard for IssueSaveCacheGuard<'_> {
    // Reads the successful request fingerprint.
    fn receipt(&self) -> Option<IssueSaveReceipt> {
        self.cache.get(&self.save_id)
    }

    // Records the fingerprint after commit.
    fn record(&self, receipt: IssueSaveReceipt) {
        self.cache.insert(self.save_id, receipt);
    }
}

/// Shared token cache charged conservatively for metadata and allocation overhead.
#[derive(Clone)]
pub struct IssueSaveCache {
    /// Bounded successful receipt fingerprints.
    tokens: Cache<Uuid, IssueSaveReceipt>,
    /// Fixed lock stripes shared by cache clones.
    locks: Arc<[Mutex<()>; 256]>,
    /// Randomized stripe selection.
    hasher: RandomState,
}

impl IssueSaveCache {
    /// Constructs an empty cache with the injected byte budget and TTL.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            tokens: Cache::builder()
                .max_capacity(
                    u64::from(config.issue_save_limit_mib.get()) * 1024 * 1024,
                )
                .time_to_live(config.issue_save_ttl)
                .weigher(|_, receipt: &IssueSaveReceipt| {
                    //
                    let bytes = receipt.created_issue_ids.iter().fold(
                        512usize,
                        |total, pair| {
                            //
                            total
                                .saturating_add(std::mem::size_of_val(pair))
                                .saturating_add(pair.local_id.capacity())
                                .saturating_add(pair.issue_id.capacity())
                        },
                    );

                    u32::try_from(bytes).unwrap_or(u32::MAX)
                })
                .build(),
            locks: Arc::new(from_fn(|_| Mutex::new(()))),
            hasher: RandomState::new(),
        }
    }

    /// Holds a fixed lock stripe across commit and receipt publication.
    pub async fn lock(&self, save_id: Uuid) -> IssueSaveCacheGuard<'_> {
        //
        let [stripe, ..] = self.hasher.hash_one(save_id).to_le_bytes();

        let [first, ..] = self.locks.as_ref();

        let guard = self
            .locks
            .get(usize::from(stripe))
            .unwrap_or(first)
            .lock()
            .await;

        IssueSaveCacheGuard {
            cache: &self.tokens,
            save_id,
            _guard: guard,
        }
    }
}

impl Default for IssueSaveCache {
    // Builds the cache with default retention.
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

impl IssueSavePort for MemCache {
    // Guard holding the receipt lock.
    type Guard<'a> = IssueSaveCacheGuard<'a>;

    // Serializes retries sharing a receipt stripe.
    fn lock_issue_save(
        &self,
        save_id: Uuid,
    ) -> impl Future<Output = Self::Guard<'_>> + Send {
        self.issue_saves.lock(save_id)
    }
}
