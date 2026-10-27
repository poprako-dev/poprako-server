//! Shared process-local cache adapter.

// Bounded annotation save receipts and exclusion guards.
mod issue_save;
// Team-partitioned online-user leases.
mod online_user;
// Bounded Unit save tokens and exclusion guards.
mod unit_save;

#[cfg(test)]
// Independent cache handle sharing and save exclusion contracts.
mod tests;

use crate::config::cache::CacheConfig;
use crate::part_impl::cache::mem_impl::issue_save::MemIssueSaveCache;
use crate::part_impl::cache::mem_impl::online_user::MemOnlineUserCache;
use crate::part_impl::cache::mem_impl::unit_save::MemUnitSaveCache;

/// Process-local state shared by clones independently of database handles.
#[derive(Clone, Default)]
pub struct MemCache {
    /// Team-partitioned online-user lease deadlines.
    online_users: MemOnlineUserCache,
    /// Save exclusion and bounded successful Unit mappings.
    unit_saves: MemUnitSaveCache,
    /// Save exclusion and bounded annotation receipts.
    issue_saves: MemIssueSaveCache,
}

impl MemCache {
    /// Builds an empty cache with the injected retention and memory budgets.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            online_users: MemOnlineUserCache::new(config.online_user_ttl),
            unit_saves: MemUnitSaveCache::new(config),
            issue_saves: MemIssueSaveCache::new(config),
        }
    }

    /// Drains cache maintenance before counting successful Unit saves in tests.
    #[cfg(test)]
    #[must_use]
    pub fn unit_save_token_count(&self) -> u64 {
        self.unit_saves.token_count()
    }
}
