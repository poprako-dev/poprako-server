//! RDB-backed chapter repository.

mod orchestra;
// Chapter step implementations (query/insert/update helpers).
mod step_impl;

/// Chapter RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;
