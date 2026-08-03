//! Shared RDB infrastructure for production adapters and extras.

use std::sync::Arc;

use anyhow::Context as _;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use tracing::instrument;

use self::result::{pool_build, pool_get};
use crate::result::{BaseError, BaseRest, accept};

/// Result helpers for Diesel-backed shared internals.
pub mod result;
/// RDB test-container helpers.
#[cfg(all(
    test,
    feature = "rdb",
    any(feature = "prom_impl", feature = "repo_impl")
))]
pub mod test_rdb;

// Internal type alias for the Diesel async connection pool.
type RdbPool = Pool<AsyncPgConnection>;

/// A pooled async PostgreSQL connection obtained from the connection pool.
pub type RdbPooledConn = Object<AsyncPgConnection>;

/// Alias for the underlying Diesel async connection type.
///
/// Used as the parameter type in all free query functions so the concrete
/// connection type is centralized in one place.
pub type RdbConn = AsyncPgConnection;

/// Centralized database connection pool holder.
///
/// Wraps an `RdbPool` behind an `Arc` for shared ownership across the application.
#[derive(Clone)]
pub struct RdbCore {
    /// Arc-wrapped Diesel connection pool shared across the application.
    pool: Arc<RdbPool>,
}

impl RdbCore {
    /// Creates a connection pool by reading the `DATABASE_URL` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if `DATABASE_URL` is not set or the pool cannot be built.
    pub fn from_env() -> anyhow::Result<Self> {
        //
        let database_url = std::env::var("DATABASE_URL")
            .with_context(|| "[RdbCore::from_env] DATABASE_URL is not set")?;

        Self::from_database_url(&database_url).map_err(|err| match err {
            BaseError::Expected { message, .. }
            | BaseError::Unrecoverable { message } => {
                anyhow::anyhow!("{}", message)
            }
        })
    }

    /// Creates a connection pool from a raw database URL string.
    pub fn from_database_url(database_url: &str) -> BaseRest<Self> {
        //
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
            database_url,
        );

        let pool = Pool::builder(manager).build().map_err(pool_build)?;

        accept(Self {
            pool: Arc::new(pool),
        })
    }

    #[instrument(level = "info", skip_all)]
    /// Retrieves a pooled connection, blocking until one is available.
    pub async fn get(&self) -> BaseRest<RdbPooledConn> {
        self.pool.get().await.map_err(pool_get)
    }
}

/// Transactional context holding a single pooled connection for the duration of a transaction.
pub struct RdbContext {
    /// Diesel pooled connection used within a single transaction scope.
    conn: RdbPooledConn,
}

impl RdbContext {
    /// Builds a new context from a pooled connection.
    pub fn new(conn: RdbPooledConn) -> Self {
        Self { conn }
    }

    /// Returns a mutable reference to the underlying pooled connection.
    pub fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }
}
