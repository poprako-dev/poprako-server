//! Diesel-backed repository and prom adapter.

use async_trait::async_trait;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::{Object, Pool};

use crate::result::RootError;
use crate::util::DeriveTransactional;

pub mod entity;
pub mod error;

#[path = "../infra/repo/schema.rs"]
pub mod schema;

pub type RdbPool = Pool<AsyncPgConnection>;
pub type RdbPooledConnection = Object<AsyncPgConnection>;

pub struct RdbRepo {
    pool: RdbPool,
}

impl RdbRepo {
    pub fn new(pool: RdbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> RdbPool {
        self.pool.clone()
    }

    pub fn from_database_url(database_url: &str) -> Result<Self, RootError> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        let pool = Pool::builder(manager).build().map_err(error::pool_build)?;

        Ok(Self::new(pool))
    }

    pub async fn connection(
        &self,
        location: &'static str,
    ) -> Result<RdbPooledConnection, RootError> {
        self.pool
            .get()
            .await
            .map_err(|err| error::pool_get(location, err))
    }
}

pub struct RdbRepoTransactional;

pub struct RdbContext {
    connection: RdbPooledConnection,
}

impl RdbContext {
    pub fn new(connection: RdbPooledConnection) -> Self {
        Self { connection }
    }

    pub fn connection(&mut self) -> &mut AsyncPgConnection {
        &mut self.connection
    }
}

#[async_trait]
impl DeriveTransactional for RdbRepo {
    type Transactional = RdbRepoTransactional;

    async fn transactional(&self) -> Self::Transactional {
        RdbRepoTransactional
    }
}
