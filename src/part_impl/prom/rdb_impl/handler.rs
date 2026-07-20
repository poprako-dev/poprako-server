//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using coordinated repository operations.
//!
//! Topic dispatch routes to [`image`].

pub use base::RdbPromHandler;

/// Shared types and dispatch logic (extracted to avoid upward ancestor dependency).
mod base;
/// Prom chapter workflow handler.
mod chapter;
/// Prom image event handler.
mod image;
/// Prom invitation event handler.
mod invitation;
mod pool;
#[cfg(all(test, feature = "repo"))]
mod tests;
mod task_flow;
