//! RDB-backed assignment invitation repository.

mod step_impl;
// Orchestration logic for assignment invitation operations.
mod orchestra;

/// Assignment invitation RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;
