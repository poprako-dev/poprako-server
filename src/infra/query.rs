pub mod user;

mod entity;
mod schema;

use async_trait::async_trait;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use futures_util::future::BoxFuture;

use crate::domain::err::{DomainError, DomainResult};
use crate::domain::model::aggr::user::{User, UserCredential, UserForm};
use crate::domain::query as domain_query;
use crate::domain::query::user::UserQeury;
use crate::domain::query::{QueryError, QueryResult, TransactionRunner};

impl From<diesel::result::Error> for DomainError {
    fn from(value: diesel::result::Error) -> Self {
        match value {
            diesel::result::Error::NotFound => QueryError::NotFound.into(),
            _ => QueryError::Unrecoverable(value.to_string()).into(),
        }
    }
}

pub struct Harness {
    pool: Pool<AsyncPgConnection>,
}

pub struct TransactionHarness<'c> {
    conn: &'c mut AsyncPgConnection,
}

impl Harness {
    fn build_transaction_harness(conn: &mut AsyncPgConnection) -> TransactionHarness<'_> {
        TransactionHarness { conn }
    }
}

impl<'c> domain_query::TransactionHarness for TransactionHarness<'c> {}

#[async_trait]
impl<'c> UserQeury for TransactionHarness<'c> {
    async fn get_by_id(&mut self, id: &str) -> QueryResult<User> {
        user::get_by_id(self.conn, id).await
    }

    async fn get_credentials_by_qid(&mut self, qid: &str) -> QueryResult<UserCredential> {
        user::get_credential_by_qid(self.conn, qid).await
    }

    async fn create(&mut self, form: UserForm) -> QueryResult<User> {
        user::create(self.conn, &form).await
    }
}

#[async_trait]
impl TransactionRunner for Harness {
    type Harness<'a> = TransactionHarness<'a>;

    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Harness<'a>) -> BoxFuture<'a, DomainResult<T>> + Send,
    {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueryError::Unrecoverable(e.to_string()))?;

        conn.build_transaction()
            .run(async move |conn| {
                let mut harness = Self::build_transaction_harness(conn);
                f(&mut harness).await
            })
            .await
    }
}
