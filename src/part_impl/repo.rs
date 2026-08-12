/// In-memory repository operations used by the hybrid production adapter.
pub mod mem_impl;
/// In-memory repository adapter used by tests.
#[cfg(test)]
pub mod mock_impl;
/// RDBMS-based repository implementation using Diesel and async connections.
pub mod rdb_impl;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use crate::shared::RdbCore;

/// Hybrid repository handle backed by PostgreSQL and process-local memory.
#[derive(Clone)]
pub struct HybRepo {
    /// Shared database connection pool.
    core: RdbCore,
    /// Team-partitioned online-user lease deadlines.
    online_user_deadlines: Arc<DashMap<String, HashMap<String, Instant>>>,
}

impl HybRepo {
    /// Builds a new hybrid repository from an [`RdbCore`] connection pool.
    pub fn new(core: RdbCore) -> Self {
        //
        Self {
            core,
            online_user_deadlines: Arc::new(DashMap::new()),
        }
    }
}
