//! Diesel-backed transaction driver.

use async_trait::async_trait;
use diesel_async::{AnsiTransactionManager, TransactionManager};

use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::util::AsyncFnMark;

use crate::result::RegularError;

use super::shared_rdb::result::diesel;
use super::shared_rdb::{RdbContext, RdbShared};

pub struct RdbDrive {
    shared: RdbShared,
}

impl RdbDrive {
    pub fn new(shared: RdbShared) -> Self {
        Self { shared }
    }
}

#[async_trait]
impl Drive<RdbContext> for RdbDrive {
    type Error = RegularError;

    async fn with_context<T, E, F>(&self, f: F) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut RdbContext) -> Result<T, E>
            + AsyncFnMark<&'c mut RdbContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let conn = self.shared.conn().await.map_err(DriveError::Backend)?;

        let mut rdb_context = RdbContext::new(conn);

        AnsiTransactionManager::begin_transaction(rdb_context.conn())
            .await
            .map_err(|e| DriveError::Backend(diesel(e)))?;

        match f(&mut rdb_context).await {
            Ok(value) => {
                AnsiTransactionManager::commit_transaction(rdb_context.conn())
                    .await
                    .map_err(|e| DriveError::Backend(diesel(e)))?;

                Ok(value)
            }
            Err(err) => {
                AnsiTransactionManager::rollback_transaction(rdb_context.conn())
                    .await
                    .map_err(|e| DriveError::Backend(diesel(e)))?;

                Err(DriveError::Advance(err))
            }
        }
    }
}
