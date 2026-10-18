//! Database repository batching policies.

use std::num::NonZeroU32;

use serde::Deserialize;

/// Positive batch sizes that fit Diesel's signed limits with a sentinel row.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RepoConfig {
    /// Maximum eligible Comics or Worksets claimed by one subtree sweep.
    pub subtree_sweep_batch_size: NonZeroU32,
    /// Unit links consumed per keyset query, excluding the sentinel row.
    pub unit_order_query_batch_size: NonZeroU32,
}

impl Default for RepoConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            subtree_sweep_batch_size: NonZeroU32::MIN.saturating_add(63),
            unit_order_query_batch_size: NonZeroU32::MIN.saturating_add(511),
        }
    }
}
