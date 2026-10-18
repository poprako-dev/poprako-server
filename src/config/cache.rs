//! Process-local cache retention policies.

use std::num::{NonZeroU32, NonZeroU64};
use std::time::Duration;

use anyhow::ensure;
use serde::Deserialize;

use crate::config::duration;

/// Memory and lease budgets shared by clones of one repository.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CacheConfig {
    /// Estimated memory budget for completed Unit saves, in MiB.
    pub unit_save_limit_mib: NonZeroU32,
    /// Token-count budget enforced through each entry's minimum weight.
    pub unit_save_token_budget: NonZeroU64,

    /// Lease duration after a successful online-user refresh.
    #[serde(
        rename = "online_user_ttl_secs",
        deserialize_with = "duration::deserialize"
    )]
    pub online_user_ttl: Duration,
}

impl CacheConfig {
    /// Returns the estimated byte capacity without losing MiB precision.
    #[must_use]
    pub fn unit_save_limit_bytes(self) -> u64 {
        u64::from(self.unit_save_limit_mib.get()) * 1024 * 1024
    }

    /// Derives the minimum entry charge from both configured budgets.
    #[must_use]
    pub fn unit_save_min_entry_weight(self) -> u64 {
        //
        self.unit_save_limit_bytes()
            .div_ceil(self.unit_save_token_budget.get())
    }

    /// Rejects unrepresentable cache weights and immediately expired leases.
    ///
    /// # Errors
    /// Returns an error for an invalid weight or a zero lease duration.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        ensure!(
            u32::try_from(self.unit_save_min_entry_weight()).is_ok(),
            "cache Unit save minimum entry weight exceeds the cache weight range"
        );

        ensure!(
            !self.online_user_ttl.is_zero(),
            "cache.online_user_ttl_secs must be positive"
        );

        Ok(())
    }
}

impl Default for CacheConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            unit_save_limit_mib: NonZeroU32::MIN.saturating_add(255),
            unit_save_token_budget: NonZeroU64::MIN.saturating_add(499_999),
            online_user_ttl: Duration::from_mins(10),
        }
    }
}
