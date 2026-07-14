//! Diesel-backed transaction driver.

use diesel_async::{AnsiTransactionManager, TransactionManager};
use poprako_orchestra::Nucl;
use poprako_orchestra::nucl::Error as NuclError;

use tracing::instrument;

use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbContext, RdbCore};
use crate::result::RegularError;

/// Diesel-backed transaction driver that wraps operations in database transactions.
///
/// Each call to [`Nucl::coord`] opens a new connection, begins a transaction,
/// runs the closure, and commits or rolls back on success or failure.
pub struct RdbDrive {
    core: RdbCore,
}

impl RdbDrive {
    /// Builds a new `RdbDrive` from an [`RdbCore`] connection pool.
    pub fn new(core: RdbCore) -> Self {
        Self { core }
    }
}

impl Nucl for RdbDrive {
    type Error = RegularError;

    type Context = RdbContext;

    #[instrument(level = "info", skip_all)]
    async fn coord<F, T, E>(&self, f: F) -> Result<T, NuclError<Self::Error, E>>
    where
        F: for<'cx> AsyncFnOnce(&'cx mut Self::Context) -> Result<T, E> + Send,
        T: Send,
        E: Send,
    {
        let conn = self.core.get().await.map_err(NuclError::Backend)?;

        let mut rdb_context = RdbContext::new(conn);

        AnsiTransactionManager::begin_transaction(rdb_context.conn())
            .await
            .map_err(|error| NuclError::Backend(diesel(error)))?;

        match f(&mut rdb_context).await {
            //
            Ok(value) => {
                //
                AnsiTransactionManager::commit_transaction(rdb_context.conn())
                    .await
                    .map_err(|error| NuclError::Backend(diesel(error)))?;

                Ok(value)
            }

            Err(error) => {
                //
                AnsiTransactionManager::rollback_transaction(
                    rdb_context.conn(),
                )
                .await
                .map_err(|error| NuclError::Backend(diesel(error)))?;

                Err(NuclError::Step(error))
            }
        }
    }
}
