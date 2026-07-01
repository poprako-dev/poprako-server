//! Shared Diesel-backed repository internals.

use std::sync::Arc;

use diesel::result::Error as DieselError;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::PoolError;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use serde_json::Error as SerdeJsonError;

use crate::result::RootError;

pub(super) mod error;

type RdbPool = Pool<AsyncPgConnection>;
type RdbPooledConn = Object<AsyncPgConnection>;

#[derive(Clone)]
pub struct RdbShared {
    pool: Arc<RdbPool>,
}

impl RdbShared {
    pub fn from_database_url(database_url: &str) -> Result<Self, RootError> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        let pool = Pool::builder(manager)
            .build()
            .map_err(|err| error::pool_build("RdbShared::from_database_url", err))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub(super) async fn conn(&self, location: &'static str) -> Result<RdbConn, RootError> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|err| pool_get(location, err))?;

        Ok(RdbConn::new(conn))
    }
}

pub(super) struct RdbConn {
    conn: RdbPooledConn,
}

impl RdbConn {
    pub(super) fn new(conn: RdbPooledConn) -> Self {
        Self { conn }
    }

    pub(super) fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }
}

pub struct RdbContext {
    rdb_conn: RdbConn,
}

impl RdbContext {
    pub(super) fn new(rdb_conn: RdbConn) -> Self {
        Self { rdb_conn }
    }

    pub(super) fn conn(&mut self) -> &mut AsyncPgConnection {
        self.rdb_conn.conn()
    }
}

pub(super) fn pool_get(location: &'static str, err: PoolError) -> RootError {
    error::pool_get(location, err)
}

pub(super) fn diesel(location: &'static str, err: DieselError) -> RootError {
    error::diesel(location, err)
}

pub(super) fn serde(location: &'static str, err: SerdeJsonError) -> RootError {
    error::serde(location, err)
}

pub(super) fn expected(message: &str) -> RootError {
    error::expected(message)
}

pub(super) fn invalid_stored_value(
    location: &'static str,
    value: impl std::fmt::Display,
) -> RootError {
    error::invalid_stored_value(location, value)
}
