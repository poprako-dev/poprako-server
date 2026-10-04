// Maps delivered tasks to domain use cases.
/// Business payload dispatch and delivery policy.
pub mod dispatch;

/// Mock prom adapter for tests.
#[cfg(test)]
pub mod mock_impl;

/// RDBMS-based prom implementation with local message queue.
pub mod rdb_impl;
