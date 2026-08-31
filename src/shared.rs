//! Shared RDB infrastructure for production adapters and extras.

/// Result helpers for Diesel-backed shared internals.
pub mod result;

#[cfg(all(
    test,
    feature = "rdb",
    any(feature = "prom_impl", feature = "repo_impl")
))]
pub mod test_rdb;

use poprako_rdb_core::RdbContext as BaseRdbContext;

use crate::part::nucl::ReptRead;

/// Application RDB context with the repository transaction level as default.
pub type RdbContext<L = ReptRead> = BaseRdbContext<L>;
