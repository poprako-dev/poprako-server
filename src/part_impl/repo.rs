/// In-memory repository operations used by the hybrid production adapter.
pub mod mem_impl;

/// In-memory repository adapter used by tests.
#[cfg(test)]
pub mod mock_impl;

/// RDBMS-based repository implementation using Diesel and async connections.
pub mod rdb_impl;

use poprako_rdb_core::RdbCore;

use crate::config::cache::CacheConfig;
use crate::config::repo::RepoConfig;
use crate::part_impl::repo::mem_impl::issue_save::IssueSaveCache;
use crate::part_impl::repo::mem_impl::online_user::OnlineUserCache;
use crate::part_impl::repo::mem_impl::unit_save::UnitSaveCache;

/// Hybrid repository handle backed by `PostgreSQL` and process-local memory.
#[derive(Clone)]
pub struct HybRepo {
    /// Shared database connection pool.
    rdb_core: RdbCore,
    /// Query and cleanup batch sizes shared by repository clones.
    config: RepoConfig,

    /// Team-partitioned online-user lease deadlines.
    online_users: OnlineUserCache,
    /// Process-local save coordination and bounded successful tokens.
    unit_saves: UnitSaveCache,
    /// Bounded annotation save receipts.
    issue_saves: IssueSaveCache,
}

impl HybRepo {
    /// Builds a new hybrid repository from an [`RdbCore`] connection pool.
    #[must_use]
    pub fn new(core: RdbCore, cache: CacheConfig, config: RepoConfig) -> Self {
        Self {
            rdb_core: core,
            config,
            online_users: OnlineUserCache::new(cache.online_user_ttl),
            unit_saves: UnitSaveCache::new(cache),
            issue_saves: IssueSaveCache::new(cache),
        }
    }
}
