//! Durable, at-least-once message delivery with application-owned handlers.
//!
//! Each payload declares its topics. One worker executes each topic serially,
//! while different topics may progress concurrently. Business commits and delivery
//! acknowledgement are independent: handlers must support replay after failed
//! acknowledgement, timeout, cancellation, or process failure. Claim tokens
//! fence delivery writes; they do not make business side effects exactly once.
//!
//! Producers use the `Prom`, `Task`, `Defer`, and `DeferBatch` contracts from
//! `poprako_orchestra_extra::prom`. Queue consumers use the actor, handler,
//! delivery, and dispatch-flow modules below.

/// Background task consumers and their lifecycle descriptors.
pub mod actor;
/// Claimed tasks and consumer delivery contracts.
pub mod delivery;
/// Dispatch outcomes and retry policy.
pub mod dispatch_flow;
/// Typed payloads, handlers, and dispatchers.
pub mod handler;

/// Typed Diesel bindings for application-owned tables and coordinators.
#[cfg(feature = "rdb_impl")]
pub mod rdb_impl;
