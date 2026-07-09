//! Repository port abstraction with transactional support.
//!
//! # The `C` type parameter
//!
//! The `C` generic parameter on repository traits is a **type-system anchor**.
//! It never appears in method signatures directly — it exists solely to
//! constrain the [`Transactional`](DeriveTransactional::Transactional)
//! associated type, ensuring that non-transactional and transactional
//! opers target the same backend session.
//!
//! This prevents, at compile time, wiring a production repository's
//! transactional handle to a mock context (or vice versa). Within a single
//! use case function scope, only one `C` implementation exists (either real
//! or mock), and the type system resolves the correct path automatically.
//!
//! # Non-transactional vs transactional opers
//!
//! Operations implemented via [`Execute`] are **non-transactional**: each
//! call uses its own database connection (obtained from a pool) and commits
//! independently. These include simple reads and single-row writes that do
//! not need atomicity with other opers.
//!
//! Operations implemented via [`Advance`] (grouped in `XxxRepoTransactional<C>`
//! traits) are **transactional**: they run inside a [`Drive::with_context`]
//! block, sharing a mutable context `C`. All advances within the same
//! block are atomic — they commit or rollback together.
//!
//! # Repository trait pattern
//!
//! Each domain has two traits:
//!
//! - `XxxRepo<C>` — the non-transactional surface. Bounds:
//!   [`DeriveTransactional`] (to obtain the transactional handle) plus
//!   [`Execute`] impls for standalone opers.
//!
//! - `XxxRepoTransactional<C>` — the transactional surface. Bounds:
//!   [`Advance`] impls for opers that must participate in a transaction.
//!
//! The non-transactional trait constrains its transactional associated type
//! with `Self::Transactional: XxxRepoTransactional<C>`, linking the two.
//!
//! [`Advance`]: poprako_transactional::advance::Advance
//! [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context

/// Announcement repository port.
pub mod announcement;
/// Assignment repository port.
pub mod assignment;
/// Assignment invitation repository port.
pub mod assignment_invitation;
/// Chapter repository port.
pub mod chapter;
/// Comic repository port.
pub mod comic;
/// Comment repository port.
pub mod comment;
/// Member repository port.
pub mod member;
/// Member invitation repository port.
pub mod member_invitation;
/// Page repository port.
pub mod page;
/// Repository step descriptors.
pub mod step;
/// System mail repository port.
pub mod system_mail;
/// Team repository port.
pub mod team;
/// Unit repository port.
pub mod unit;
/// User repository port.
pub mod user;
/// Workset repository port.
pub mod workset;
