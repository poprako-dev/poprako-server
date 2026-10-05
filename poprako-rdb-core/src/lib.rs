//! Application-neutral RDB infrastructure.

/// Neutral `PostgreSQL` pool and transaction context.
pub mod rdb;

pub use crate::rdb::{
    RdbConn, RdbContext, RdbCore, RdbError, RdbPooledConn, RdbRest,
};
