//! Crate root: public module declarations, re-exports, and internal module
//! organization for the PopRaKo application core.

/// Application configuration parsing and access.
pub mod config;

/// A forward-reference utility for resolving cyclic dependency patterns.
pub mod forward_ref;

/// Re-export of [ForwardRef] for convenience at the crate root.
pub use forward_ref::ForwardRef;

mod complex;
mod data;
mod harn;
mod model;
mod part;
mod part_impl;
mod result;
mod usecase;
mod value;

mod util;

#[cfg(test)]
mod test_util;
