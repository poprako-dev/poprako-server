// Maps delivered tasks to domain use cases.
mod dispatch;

/// Mock prom adapter for tests.
#[cfg(test)]
pub mod mock_impl;

/// RDBMS-based prom implementation with local message queue.
pub mod rdb_impl;
/// Task outcomes shared by the local-message actor and production composition.
pub mod task_flow;
