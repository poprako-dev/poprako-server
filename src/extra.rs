//! Long-lived production background jobs.

/// Generic recurring-task scheduler.
pub mod sched;
/// Relational hierarchy sweep task injected into the scheduler.
pub mod subtree_delete;
