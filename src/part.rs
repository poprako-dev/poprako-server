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
//! | [`token`]  | Authentication token signing |
//!
//! [`part_impl`]: super::part_impl

pub mod effect;
pub mod image;
pub mod prom;
pub mod repo;
pub mod token;
