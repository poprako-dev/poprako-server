/// In-memory repository adapter used by tests.
#[cfg(test)]
pub mod mock_impl;

/// RDBMS-based repository implementation using Diesel and async connections.
pub mod rdb_impl;
