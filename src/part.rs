//! Pluggable port abstractions following the ports-and-adapters pattern.
//!
//! Each sub-module defines a trait contract that the application core depends
//! on. Concrete implementations live in [`part_impl`] and can be swapped
//! independently — for example, a mock image pool in tests versus a
//! production S3-backed pool.
//!
//! # Sub-modules
//!
//! | Module | Role |
//! |--------|------|
//! | [`effect`] | Side-effect dispatch (events emitted by use cases) |
//! | [`image`]  | Object-storage signed URL resolution |
//! | [`prom`]   | Deferred actions executed after transaction commit |
//! | [`repo`]   | Persistent storage with transactional support |
//! | [`auth`]   | Authentication token signing |
//!
//! [`part_impl`]: super::part_impl

/// Authentication port — token signing and verification.
pub mod auth;
/// Side-effect dispatch port.
pub mod effect;
/// Object-storage image port — signed URL generation.
pub mod image;
/// Deferred-action port — actions executed after transaction commit.
pub mod prom;
/// Repository port — persistent storage abstractions with transactional support.
pub mod repo;

/// Shared port helper traits (drive execution, proxy execution).
pub mod shared;
