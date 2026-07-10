//! Diesel-backed transaction driver.

use async_trait::async_trait;
use diesel_async::{AnsiTransactionManager, TransactionManager};

use poprako_transactional::drive::Drive;
use poprako_transactional::drive::result::Error as DriveError;
use poprako_transactional::util::AsyncFnMark;

use crate::result::RegularError;

use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbContext, RdbCore};

/// Diesel-backed transaction driver that wraps operations in database transactions.
///
/// Each call to `with_context` opens a new connection, begins a transaction,
/// runs the closure, and commits or rolls back on success or failure.
pub struct RdbDrive {
    core: RdbCore,
}

impl RdbDrive {
    pub fn new(core: RdbCore) -> Self {
        Self { core }
    }
}

#[async_trait]
impl Drive<RdbContext> for RdbDrive {
    type Error = RegularError;

    async fn with_context<T, E, F>(
        &self,
        f: F,
    ) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut RdbContext) -> Result<T, E>
            + AsyncFnMark<&'c mut RdbContext, Result<T, E>, Fut: Send>
            + Send,
    {
        let conn = self.core.get().await.map_err(DriveError::Backend)?;

        let mut rdb_context = RdbContext::new(conn);

        // Manual begin/commit/rollback instead of `TransactionManager::transaction()`
        // because that method requires `E: From<diesel::result::Error>`, but our
        // generic advance error `E` does not satisfy this bound.
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
                AnsiTransactionManager::rollback_transaction(
                    rdb_context.conn(),
                )
                .await
                .map_err(|e| DriveError::Backend(diesel(e)))?;

                Err(DriveError::Advance(err))
            }
        }
    }
}
