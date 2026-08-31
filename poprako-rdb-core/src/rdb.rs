//! Application-neutral `PostgreSQL` pool and Orchestra context.

use std::marker::PhantomData;
use std::sync::Arc;

use anyhow::Context as _;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::{BuildError, Object, Pool};
use poprako_orchestra::{Context, Level};

// Concrete pool type shared by RDB contexts.
type RdbPool = Pool<AsyncPgConnection>;

/// A pooled `PostgreSQL` connection.
pub type RdbPooledConn = Object<AsyncPgConnection>;

/// The concrete asynchronous `PostgreSQL` connection.
pub type RdbConn = AsyncPgConnection;

/// Result returned by neutral RDB infrastructure.
pub type RdbRest<T> = Result<T, RdbError>;

/// Error exposed by reusable RDB infrastructure.
#[derive(Debug)]
pub enum RdbError {
    //
    /// The connection pool could not be constructed.
    PoolBuild {
        /// Original pool-construction failure.
        source: BuildError,
    },

    /// A pooled connection could not be acquired.
    PoolGet {
        /// Safe diagnostic from the pool implementation.
        message: String,
    },
}

impl std::fmt::Display for RdbError {
    // Formats an infrastructure failure without adding contextual classification.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //
        match self {
            //
            Self::PoolBuild { source } => {
                write!(formatter, "failed to build RDB pool: {}", source)
            }

            Self::PoolGet { message } => {
                //
                write!(
                    formatter,
                    "failed to acquire RDB connection: {}",
                    message
                )
            }
        }
    }
}

impl std::error::Error for RdbError {}

/// Shared `PostgreSQL` connection-pool owner.
#[derive(Clone)]
pub struct RdbCore {
    /// Shared asynchronous connection pool.
    pool: Arc<RdbPool>,
}

impl RdbCore {
    /// Builds a pool from `DATABASE_URL`.
    ///
    /// # Errors
    ///
    /// Returns an error when the environment variable is absent or the pool
    /// cannot be built.
    pub fn from_env() -> anyhow::Result<Self> {
        //
        let database_url = std::env::var("DATABASE_URL")
            .context("[RdbCore::from_env] DATABASE_URL is not set")?;

        Self::from_database_url(&database_url).map_err(anyhow::Error::new)
    }

    /// Builds a pool for one database URL.
    ///
    /// # Errors
    ///
    /// Returns an error when the connection pool cannot be built.
    pub fn from_database_url(database_url: &str) -> RdbRest<Self> {
        //
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
            database_url,
        );

        let pool = Pool::builder(manager)
            .build()
            .map_err(|source| RdbError::PoolBuild { source })?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Acquires one pooled connection.
    ///
    /// # Errors
    ///
    /// Returns an error when the pool cannot supply a connection.
    pub async fn get(&self) -> RdbRest<RdbPooledConn> {
        //
        self.pool.get().await.map_err(|source| RdbError::PoolGet {
            message: source.to_string(),
        })
    }
}

/// Transaction context carrying one pooled `PostgreSQL` connection.
pub struct RdbContext<L> {
    //
    /// Pooled connection used by this transaction context.
    conn: RdbPooledConn,

    /// Compile-time Orchestra transaction level.
    level: PhantomData<L>,
}

impl<L> RdbContext<L> {
    /// Builds a context from one acquired connection.
    pub const fn new(conn: RdbPooledConn) -> Self {
        //
        Self {
            conn,
            level: PhantomData,
        }
    }

    /// Returns the transaction connection.
    pub fn conn(&mut self) -> &mut RdbConn {
        &mut self.conn
    }
}

impl<L> Context for RdbContext<L>
where
    L: Level,
{
    // Declares the context transaction level.
    type Level = L;
}
