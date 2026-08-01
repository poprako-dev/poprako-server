//! Diesel-backed transaction coordinator.

use diesel_async::{AnsiTransactionManager, TransactionManager};
use poprako_orchestra::Nucl;
use poprako_orchestra::nucl::Error as NuclError;
use tracing::instrument;

use crate::result::BaseError;
use crate::shared::result::diesel;
use crate::shared::{RdbContext, RdbCore};

/// Diesel-backed transaction coordinator that wraps operations in database transactions.
///
/// Each call to [`Nucl::coord`] opens a new connection, begins a transaction,
/// runs the closure, and commits or rolls back on success or failure.
pub struct RdbNucl {
    /// Shared database connection pool used for transactions.
    core: RdbCore,
}

impl RdbNucl {
    /// Builds a new `RdbNucl` from an [`RdbCore`] connection pool.
    pub fn new(core: RdbCore) -> Self {
        Self { core }
    }
}

impl Nucl for RdbNucl {
    // Transaction error type propagated through the Nucl trait.
    type Error = BaseError;

    // Transaction context wrapping a pooled Diesel connection.
    type Context = RdbContext;

    // Coordinates a closure within a database transaction, committing on success
    // and rolling back on error.
    #[instrument(level = "info", skip_all)]
    async fn coord<F, T, E>(&self, f: F) -> Result<T, NuclError<Self::Error, E>>
    where
        F: AsyncFnOnce(&mut Self::Context) -> Result<T, E> + Send,
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
