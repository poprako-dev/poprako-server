//! Pluggable port abstractions following the ports-and-adapters pattern.
//!
//! Each sub-module defines a trait contract that the application core depends
//! on. Concrete implementations live in [`part_impl`] and can be swapped
//! independently — for example, an in-memory repository in tests versus a
//! production `PostgreSQL` repository.
//!
//! # Sub-modules
//!
//! | Module | Role |
//! |--------|------|
//! | [`effect`] | Side-effect dispatch (events emitted by use cases) |
//! | [`obj_dept`] | Business-object storage operations |
//! | [`nucl`]   | Transaction-coordinator boundary error conversion |
//! | [`prom`]   | Deferred actions executed after transaction commit |
//! | [`repo`]   | Persistent storage with transactional support |
//! | [`auth`]   | Authentication token signing |
//!
//! [`part_impl`]: super::part_impl

/// Authentication port — token signing and verification.
pub mod auth;
/// Side-effect dispatch port.
pub mod effect;
/// Transaction-coordinator boundary error conversion.
pub mod nucl;
/// Business-object marker types used by the total ObjDept.
pub mod obj_dept;
/// Deferred-action port — actions executed after transaction commit.
pub mod prom;
/// Repository port — persistent storage abstractions with transactional support.
pub mod repo;
