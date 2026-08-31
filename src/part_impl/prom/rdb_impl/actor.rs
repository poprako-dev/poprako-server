//! Prom-consumer worker that polls, dispatches by topic shard, and executes
//! deferred actions through statically typed business dependencies.

// Spawns the fixed worker pool and coordinates row dispatch.
mod pool;

/// Shared types and dispatch logic.
pub mod base;

/// RDB prom actor integration tests.
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
pub mod tests;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
use crate::part_impl::prom::task_flow;
