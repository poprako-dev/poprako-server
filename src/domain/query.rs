use async_trait::async_trait;
use futures_util::future::BoxFuture;

use crate::domain::err::DomainResult;
use crate::domain::query::user::UserQeury;
use crate::util::rename::StdResult;

pub mod user;

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("unrecoverable: {0}")]
    Unrecoverable(String),
}

pub type QueryResult<T> = StdResult<T, QueryError>;

pub trait TransactionHarness: Send + UserQeury {}

#[async_trait]
pub trait TransactionRunner: Send + Sync {
    // Context is the provider that gives out queries with transaction context injected.
    type Harness<'a>: TransactionHarness + 'a
    where
        Self: 'a;

    // run_in_transaction runs the given function in a transaction, and returns the result of the function.
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Harness<'a>) -> BoxFuture<'a, DomainResult<T>> + Send;
}
