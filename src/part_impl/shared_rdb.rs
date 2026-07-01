//! Shared Diesel-backed repository internals.

use std::sync::Arc;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::{Object, Pool};

use crate::result::RegularResult;

pub mod result;

use self::result::{pool_build, pool_get};

type RdbPool = Pool<AsyncPgConnection>;

pub type RdbPooledConn = Object<AsyncPgConnection>;

#[derive(Clone)]
pub struct RdbShared {
    pool: Arc<RdbPool>,
}

impl RdbShared {
    pub fn from_database_url(database_url: &str) -> RegularResult<Self> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        let pool = Pool::builder(manager).build().map_err(pool_build)?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub(super) async fn get(&self) -> RegularResult<RdbPooledConn> {
        self.pool.get().await.map_err(pool_get)
    }
}

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
