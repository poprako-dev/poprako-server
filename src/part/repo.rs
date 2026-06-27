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

pub mod comic;
pub mod member;
pub mod member_invitation;
pub mod step;
pub mod system_mail;
pub mod team;
pub mod user;
pub mod workset;

/// Executes a single non-transactional [`Step`] against the repository.
/// It is designed to keep consistency with `Advance`.
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

pub mod proxy {
    use async_trait::async_trait;

    use poprako_transactional::advance::Advance;
    use poprako_transactional::step::Step;

    use crate::part::repo::Execute;

    /// Executes a single [`Step`] against a repository via a mutable proxy
    /// reference.
    ///
    /// Unlike [`Execute`] (which uses `&self`), this trait requires
    /// `&mut self`, allowing proxy types to carry mutable context — a
    /// transactional-handle + context pair — and dispatch to either
    /// [`Execute`] or [`Advance`] depending on the proxy variant.
    #[async_trait]
    pub trait ProxyExecute<S>
    where
        S: Step,
    {
        type Error;

        /// Executes a step through the proxy.
        async fn execute(&mut self, step: &S) -> Result<S::Output, Self::Error>;
    }

    /// Views a repository reference as a non-transactional proxy.
    pub trait AsProxyNonTransactional {
        /// Wraps this repository reference as a non-transactional proxy.
        fn as_proxy(&self) -> ProxyNonTransactional<'_, Self>
        where
            Self: Sized,
        {
            ProxyNonTransactional::new(self)
        }
    }

    impl<R> AsProxyNonTransactional for R {}

    /// A non-transactional proxy that delegates to [`Execute`].
    ///
    /// Created locally at the usecase call-site; wraps a shared reference to
    /// the repository and forwards every [`ProxyExecute::execute`] call to
    /// [`Execute::execute`].
    pub struct ProxyNonTransactional<'a, R> {
        repo: &'a R,
    }

    impl<'a, R> ProxyNonTransactional<'a, R> {
        /// Wraps a shared repository reference for permission-check
        /// dispatching.
        pub fn new(repo: &'a R) -> Self {
            Self { repo }
        }
    }

    #[async_trait]
    impl<'a, R, S> ProxyExecute<S> for ProxyNonTransactional<'a, R>
    where
        R: Execute<S> + Sync,
        S: Step + Sync,
    {
        type Error = R::Error;

        async fn execute(&mut self, step: &S) -> Result<S::Output, Self::Error> {
            Execute::execute(self.repo, step).await
        }
    }

    /// A transactional proxy that delegates to [`Advance`].
    ///
    /// Created locally inside a [`Drive::with_context`] closure; wraps a
    /// transactional-handle reference together with a mutable context
    /// reference and forwards every [`ProxyExecute::execute`] call to
    /// [`Advance::advance`].
    ///
    /// [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context
    pub struct ProxyTransactional<'a, R, C> {
        repo: &'a R,
        context: &'a mut C,
    }

    /// Views a transactional handle reference as a transactional proxy.
    pub trait AsProxyTransactional<C> {
        /// Wraps this transactional handle and context as a transactional proxy.
        fn as_proxy<'a>(&'a self, context: &'a mut C) -> ProxyTransactional<'a, Self, C>
        where
            Self: Sized,
        {
            ProxyTransactional::new(self, context)
        }
    }

    impl<R, C> AsProxyTransactional<C> for R {}

    impl<'a, R, C> ProxyTransactional<'a, R, C> {
        /// Wraps a transactional-handle reference + mutable context reference
        /// for permission-check dispatching.
        pub fn new(repo: &'a R, context: &'a mut C) -> Self {
            Self { repo, context }
        }
    }

    #[async_trait]
    impl<'a, R, C, S> ProxyExecute<S> for ProxyTransactional<'a, R, C>
    where
        R: Advance<S, C> + Sync,
        C: Send,
        S: Step + Sync,
    {
        type Error = R::Error;

        async fn execute(&mut self, step: &S) -> Result<S::Output, Self::Error> {
            Advance::advance(self.repo, self.context, step).await
        }
    }
}
