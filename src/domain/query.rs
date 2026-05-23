use async_trait::async_trait;
use futures_util::future::BoxFuture;

use crate::domain::err::DomainRetVal;
use crate::domain::query::user::UserQeuryMut;
use crate::util::rename::StdRetVal;

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

pub type QueryRetVal<T> = StdRetVal<T, QueryError>;

pub trait TransactionalQuery: Send + UserQeuryMut {}

#[async_trait]
pub trait Transactional: Send + Sync {
    // Context is the provider that gives out queries with transaction context injected.
    type Query<'a>: TransactionalQuery + 'a
    where
        Self: 'a;

    // run_in_transaction runs the given function in a transaction, and returns the result of the function.
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainRetVal<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainRetVal<T>> + Send;
}
