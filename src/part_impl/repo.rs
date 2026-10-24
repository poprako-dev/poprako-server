/// In-memory repository adapter used by tests.
#[cfg(test)]
pub mod mock_impl;

/// RDBMS-based repository implementation using Diesel and async connections.
pub mod rdb_impl;

use poprako_rdb_core::RdbCore;

use crate::config::repo::RepoConfig;

/// Repository handle backed by `PostgreSQL`.
#[derive(Clone)]
pub struct RdbRepo {
    /// Shared database connection pool.
    rdb_core: RdbCore,
    /// Query and cleanup batch sizes shared by repository clones.
    config: RepoConfig,
}

impl RdbRepo {
    /// Builds a database repository with the injected batching policy.
    #[must_use]
    pub const fn new(core: RdbCore, config: RepoConfig) -> Self {
        Self {
            rdb_core: core,
            config,
        }
    }
}
