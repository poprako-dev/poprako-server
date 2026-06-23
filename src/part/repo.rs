//! Repository port abstraction with transactional support.
//!
//! # The `C` type parameter
//!
//! The `C` generic parameter on repository traits is a **type-system anchor**.
//! It never appears in method signatures directly — it exists solely to
//! constrain the [`Transactional`](DeriveTransactional::Transactional)
//! associated type, ensuring that non-transactional and transactional
//! operations target the same backend session.
//!
//! This prevents, at compile time, wiring a production repository's
//! transactional handle to a mock context (or vice versa). Within a single
//! use case function scope, only one `C` implementation exists (either real
//! or mock), and the type system resolves the correct path automatically.
//!
//! # Non-transactional vs transactional operations
//!
//! Operations implemented via [`Execute`] are **non-transactional**: each
//! call uses its own database connection (obtained from a pool) and commits
//! independently. These include simple reads and single-row writes that do
//! not need atomicity with other operations.
//!
//! Operations implemented via [`Advance`] (grouped in `XxxRepoTransactional<C>`
//! traits) are **transactional**: they run inside a [`Drive::with_context`]
//! closure, sharing a mutable context `C`. All advances within the same
//! closure are atomic — they commit or rollback together.
//!
//! # Repository trait pattern
//!
//! Each domain has two traits:
//!
//! - `XxxRepo<C>` — the non-transactional surface. Bounds:
//!   [`DeriveTransactional`] (to obtain the transactional handle) plus
//!   [`Execute`] impls for standalone operations.
//!
//! - `XxxRepoTransactional<C>` — the transactional surface. Bounds:
//!   [`Advance`] impls for operations that must participate in a transaction.
//!
//! The non-transactional trait constrains its transactional associated type
//! with `Self::Transactional: XxxRepoTransactional<C>`, linking the two.
//!
//! [`Advance`]: poprako_transactional::advance::Advance
//! [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context

use async_trait::async_trait;

use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::step::Step;

use crate::result::RootError;

pub mod member;
pub mod member_invitation;
pub mod step;
pub mod system_mail;
pub mod team;
pub mod user;
pub mod workset;

/// Executes a single non-transactional [`Step`] against the repository.
///
/// Each `execute` call uses its own database connection and commits
/// independently. For operations that must be atomic with other steps,
/// use the transactional [`Advance`] path instead.
///
/// [`Advance`]: poprako_transactional::advance::Advance
#[async_trait]
pub trait Execute<S>
where
    S: Step,
{
    type Error;

    async fn execute(&self, step: &S) -> Result<S::Output, Self::Error>;
}

/// Converts a [`DriveError`] into a [`RootError`].
///
/// This utility maps transaction-driver errors (which carry both an
/// operation error `E` and a finalizer/commit error `BE`) into the
/// application's unified error type.
pub fn map_drive_err<E, BE>(err: DriveError<E, BE>) -> RootError
where
    E: Into<RootError>,
    BE: Into<RootError>,
{
    err.into()
}
