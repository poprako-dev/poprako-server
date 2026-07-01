//! Shared Diesel-backed repository internals.

use std::sync::Arc;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::{Object, Pool};

use crate::result::RegularError;

pub mod result;

use self::result::{pool_build, pool_get};

type RdbPool = Pool<AsyncPgConnection>;
type RdbPooledConn = Object<AsyncPgConnection>;

#[derive(Clone)]
pub struct RdbShared {
    pool: Arc<RdbPool>,
}

impl RdbShared {
    pub fn from_database_url(database_url: &str) -> Result<Self, RegularError> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        let pool = Pool::builder(manager).build().map_err(pool_build)?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn conn(&self) -> Result<RdbConn, RegularError> {
        let conn = self.pool.get().await.map_err(pool_get)?;

        Ok(RdbConn::new(conn))
    }
}

pub struct RdbConn {
    conn: RdbPooledConn,
}

impl RdbConn {
    pub fn new(conn: RdbPooledConn) -> Self {
        Self { conn }
    }

    pub fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }
}

pub struct RdbContext {
    rdb_conn: RdbConn,
}

impl RdbContext {
    pub fn new(rdb_conn: RdbConn) -> Self {
        Self { rdb_conn }
    }

    pub fn conn(&mut self) -> &mut AsyncPgConnection {
        self.rdb_conn.conn()
    }
}
