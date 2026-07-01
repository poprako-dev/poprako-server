//! Diesel-backed transaction driver.

use async_trait::async_trait;
use diesel_async::{AnsiTransactionManager, TransactionManager};

use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::util::AsyncFnMark;

use crate::part_impl::rdb_repo::{RdbContext, RdbPool, error};
use crate::result::RootError;

pub struct RdbDrive {
    pool: RdbPool,
}

impl RdbDrive {
    pub fn new(pool: RdbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Drive<RdbContext> for RdbDrive {
    type Error = RootError;

    async fn with_context<T, E, F>(&self, f: F) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut RdbContext) -> Result<T, E>
            + AsyncFnMark<&'c mut RdbContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let connection =
            self.pool.get().await.map_err(|err| {
                DriveError::Backend(error::pool_get("RdbDrive::with_context", err))
            })?;

        let mut rdb_context = RdbContext::new(connection);

        AnsiTransactionManager::begin_transaction(rdb_context.connection())
            .await
            .map_err(|err| DriveError::Backend(error::diesel(err)))?;

        let result = f(&mut rdb_context).await;

        match result {
            Ok(value) => {
                AnsiTransactionManager::commit_transaction(rdb_context.connection())
                    .await
                    .map_err(|err| DriveError::Backend(error::diesel(err)))?;

                Ok(value)
            }
            Err(err) => {
                AnsiTransactionManager::rollback_transaction(rdb_context.connection())
                    .await
                    .map_err(|rollback_err| DriveError::Backend(error::diesel(rollback_err)))?;

                Err(DriveError::Advance(err))
            }
        }
    }
}
