//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using coordinated repository operations.
//!
//! Topic dispatch routes to business workflow actors.

// Prom chapter workflow actor.
mod chapter;
// Prom invitation event actor.
mod invitation;
// Spawns one worker per topic and coordinates row dispatch.
mod pool;
// Orchestrates the two-step task routing/cleanup flow for each topic.
mod task_flow;

/// Shared types and dispatch logic.
pub mod base;

/// RDB prom actor integration tests.
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
pub mod tests;
