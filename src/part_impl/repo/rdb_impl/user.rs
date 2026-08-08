//! RDB-backed user repository — free query functions and thin trait impls.

// User repository impl blocks.
mod impls;
// ── Free functions ──────────────────────────────────────────────────────────

// Remove a user row from persistence.

// User free-function helpers.
mod helpers;

/// User RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;
