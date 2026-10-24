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
use crate::part_impl::cache::mem_impl::issue_save::IssueSaveCache;
use crate::part_impl::cache::mem_impl::online_user::OnlineUserCache;
use crate::part_impl::cache::mem_impl::unit_save::UnitSaveCache;

/// Process-local state shared by clones independently of database handles.
#[derive(Clone, Default)]
pub struct MemCache {
    /// Team-partitioned online-user lease deadlines.
    online_users: OnlineUserCache,
    /// Save exclusion and bounded successful Unit mappings.
    unit_saves: UnitSaveCache,
    /// Save exclusion and bounded annotation receipts.
    issue_saves: IssueSaveCache,
}

impl MemCache {
    /// Builds an empty cache with the injected retention and memory budgets.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            online_users: OnlineUserCache::new(config.online_user_ttl),
            unit_saves: UnitSaveCache::new(config),
            issue_saves: IssueSaveCache::new(config),
        }
    }

    /// Drains cache maintenance before counting successful Unit saves in tests.
    #[cfg(test)]
    #[must_use]
    pub fn unit_save_token_count(&self) -> u64 {
        self.unit_saves.token_count()
    }
}
