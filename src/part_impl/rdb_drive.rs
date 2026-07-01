//! Diesel-backed transaction driver.

use async_trait::async_trait;
use diesel_async::{AnsiTransactionManager, TransactionManager};

use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::util::AsyncFnMark;

use crate::result::RootError;

use super::rdb_shared::{self, RdbContext, RdbShared};

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
    type Error = RootError;

    async fn with_context<T, E, F>(&self, f: F) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut RdbContext) -> Result<T, E>
            + AsyncFnMark<&'c mut RdbContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let conn = self
            .shared
            .conn("RdbDrive::with_context")
            .await
            .map_err(DriveError::Backend)?;

        let mut rdb_context = RdbContext::new(conn);

        AnsiTransactionManager::begin_transaction(rdb_context.conn())
            .await
            .map_err(|err| {
                DriveError::Backend(rdb_shared::diesel("RdbDrive::with_context begin", err))
            })?;

        let result = f(&mut rdb_context).await;

        match result {
            Ok(value) => {
                AnsiTransactionManager::commit_transaction(rdb_context.conn())
                    .await
                    .map_err(|err| {
                        DriveError::Backend(rdb_shared::diesel(
                            "RdbDrive::with_context commit",
                            err,
                        ))
                    })?;

                Ok(value)
            }
            Err(err) => {
                AnsiTransactionManager::rollback_transaction(rdb_context.conn())
                    .await
                    .map_err(|rollback_err| {
                        DriveError::Backend(rdb_shared::diesel(
                            "RdbDrive::with_context rollback after advance error",
                            rollback_err,
                        ))
                    })?;

                Err(DriveError::Advance(err))
            }
        }
    }
}
