//! Shared Diesel-backed repository internals.

use std::sync::Arc;

use anyhow::Context as _;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::{Object, Pool};

use crate::result::{RegularError, RegularResult};

/// Result helpers for Diesel-backed shared internals.
pub mod result;

use self::result::{pool_build, pool_get};

/// Internal type alias for the Diesel async connection pool.
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
            RegularError::Expected { message, .. }
            | RegularError::Unrecoverable { message } => {
                anyhow::anyhow!("{}", message)
            }
        })
    }

    pub fn from_database_url(database_url: &str) -> RegularResult<Self> {
        //
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
            database_url,
        );

        let pool = Pool::builder(manager).build().map_err(pool_build)?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn get(&self) -> RegularResult<RdbPooledConn> {
        self.pool.get().await.map_err(pool_get)
    }
}

/// Transactional context holding a single pooled connection for the duration of a transaction.
pub struct RdbContext {
    conn: RdbPooledConn,
}

impl RdbContext {
    pub fn new(conn: RdbPooledConn) -> Self {
        Self { conn }
    }

    pub fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }
}
