pub mod user;

mod entity;
mod schema;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use futures_util::future::BoxFuture;

use crate::domain::err::{DomainError, DomainRetVal};
use crate::domain::query as domain_query;
use crate::domain::query::{QueryError, Transactional};

impl From<diesel::result::Error> for QueryError {
    fn from(val: diesel::result::Error) -> Self {
        match val {
            diesel::result::Error::NotFound => QueryError::NotFound,
            _ => QueryError::Unrecoverable(val.to_string()),
        }
    }
}

impl From<diesel::result::Error> for DomainError {
    fn from(val: diesel::result::Error) -> Self {
        QueryError::from(val).into()
    }
}

pub struct Query {
    pool: Pool<AsyncPgConnection>,
}

impl Query {
    fn build_transactional_query(conn: &mut AsyncPgConnection) -> TransactionalQuery<'_> {
        TransactionalQuery::new(conn)
    }
}

#[async_trait::async_trait]
impl Transactional for Query {
    type Query<'a> = TransactionalQuery<'a>;

    async fn run_in_transaction<F, T>(&self, f: F) -> DomainRetVal<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainRetVal<T>> + Send,
    {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueryError::Unrecoverable(e.to_string()))?;

        conn.build_transaction()
            .run(async move |conn| {
                let mut harness = Self::build_transactional_query(conn);
                f(&mut harness).await
            })
            .await
    }
}

pub struct TransactionalQuery<'c> {
    conn: &'c mut AsyncPgConnection,
}

impl<'c> domain_query::TransactionalQuery for TransactionalQuery<'c> {}

impl<'c> TransactionalQuery<'c> {
    pub fn new(conn: &'c mut AsyncPgConnection) -> Self {
        Self { conn }
    }
}
