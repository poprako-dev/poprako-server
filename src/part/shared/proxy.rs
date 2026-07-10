use async_trait::async_trait;

use poprako_transactional::advance::Advance;
use poprako_transactional::step::Step;

use crate::part::shared::execute::Execute;

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
        self.repo.execute(step).await
    }
}

/// A transactional proxy that delegates to [`Advance`].
///
/// Created locally inside a [`Drive::with_context`] block; wraps a
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
    fn as_proxy<'a>(
        &'a self,
        context: &'a mut C,
    ) -> ProxyTransactional<'a, Self, C>
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
        self.repo.advance(self.context, step).await
    }
}
