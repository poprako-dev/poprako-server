//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using coordinated repository operations.
//!
//! Topic dispatch routes to [`image`].

// Shared types and dispatch logic (extracted to avoid upward ancestor dependency).
mod base;
// Prom chapter workflow handler.
mod chapter;
// Prom image event handler.
mod image;
// Prom invitation event handler.
mod invitation;
// Spawns one worker per topic and coordinates row dispatch.
mod pool;
// Orchestrates the two-step task routing/cleanup flow for each topic.
mod task_flow;

/// RDB prom handler integration tests.
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
pub mod tests;

pub use crate::part_impl::prom::rdb_impl::handler::base::RdbPromHandler;
