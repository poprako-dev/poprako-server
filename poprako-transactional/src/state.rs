use async_trait::async_trait;

use crate::{handle::Handle, step::Step};

#[async_trait]
pub trait StateAdvance<S>
where
    S: Step,
{
    // Executing the step with the given state.
    // It wraps the state and step together to simplify calls of Advance.
    async fn advance(&mut self, step: S) -> Result<S::Output, S::Error>;
}

#[async_trait]
pub trait StateTransactional {
    /// The error type of the state transactional operations.
    type Error;

    /// Commits the state transactional operations.
    async fn commit(self) -> Result<(), Self::Error>;

    /// Rollbacks the state transactional operations.
    async fn rollback(self) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait Backend {
    // TODO: comment.
    type Handle: Handle;

    /// Error type of beginning a transaction.
    type Error;

    async fn begin(&self) -> Result<Self::Handle, Self::Error>;
}
