pub mod member;
pub mod member_invitation;
pub mod user;

mod entity;
mod schema;

/// Allocates a connection from the pool, mapping pool errors to
/// [`DomainError::Unrecoverable`].  Prefer [`execute_query`] in `Query`
/// impl blocks; this macro is for call sites that need the raw
/// [`deadpool::managed::Object`] (e.g. `run_in_transaction`).
#[macro_export]
macro_rules! allocate_connection {
    ($pool:expr, $loc:expr) => {
        $pool
            .get()
            .await
            .map_err(|e| {
                $crate::domain::result::DomainError::unrecoverable(format!(
                    "[{}] error getting connection: {}",
                    $loc, e
                ))
            })
            .trace_error()?
    };
}

/// Allocates a connection and immediately calls a query function on it.
///
/// The query function must accept `(&mut AsyncPgConnection, args...)` and
/// return a `Future<Output = DomainResult<_>>`.
///
/// # Examples
///
/// ```ignore
/// execute_query!(self.pool, get_credential_by_qid, qid)
/// execute_query!(self.pool, get_by_id, id)
/// ```
///
/// The macro expands to:
/// ```ignore
/// let mut conn = allocate_connection!(self.pool, "Query::get_by_id");
/// get_by_id(conn.as_mut(), id).await
/// ```
#[macro_export]
macro_rules! execute_query {
    ($pool:expr, $fn:path $(, $arg:expr)* $(,)?) => {{
        let mut conn = $crate::allocate_connection!($pool, concat!("Query::", stringify!($fn)));
        $fn(conn.as_mut(), $($arg),*).await
    }};
}

use anyhow::Context as _;
use async_trait::async_trait;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use futures_util::future::BoxFuture;

use tracing::Level;
use tracing::instrument;

use crate::domain::query::Transactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::util::err::ErrorTrace as _;

impl From<diesel::result::Error> for DomainError {
    fn from(val: diesel::result::Error) -> Self {
        // NotFound is handled by each function with a contextual message; the rest become Unrecoverable.
        let err = DomainError::unrecoverable(val.to_string());
        tracing::error!("[trace_error] {}", err);
        err
    }
}

pub struct Query {
    pool: Pool<AsyncPgConnection>,
}

impl Query {
    pub async fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .with_context(|| "[Query::from_env] DATABASE_URL is not set")?;

        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        let pool = Pool::builder(manager)
            .build()
            .with_context(|| "[Query::from_env] failed to build connection pool")?;

        tracing::debug!("[Query::from_env] configured");

        Ok(Self { pool })
    }

    fn build_transactional_query(conn: &mut AsyncPgConnection) -> QueryTransactional<'_> {
        QueryTransactional { conn }
    }
}

#[async_trait]
impl Transactional for Query {
    type Query<'a> = QueryTransactional<'a>;

    #[instrument(skip(self, f), level = Level::DEBUG)]
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send, // Return value must cross .await boundaries; Tokio multi-threaded runtime requires Send
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResult<T>> + Send, // BoxFuture requires the closure to be Send
    {
        let mut conn = allocate_connection!(self.pool, "Query::run_in_transaction");

        conn.build_transaction()
            .run(async move |conn| {
                let mut harness = Self::build_transactional_query(conn);
                f(&mut harness).await
            })
            .await
    }
}

pub struct QueryTransactional<'c> {
    conn: &'c mut AsyncPgConnection,
}
