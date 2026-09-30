//! Diesel-backed transaction coordinator.

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
mod tests;

use std::marker::PhantomData;

use diesel::connection::TransactionManagerStatus;
use diesel::define_sql_function;
use diesel::sql_types::{Bool, Text};
use diesel_async::{
    AnsiTransactionManager, RunQueryDsl as _, TransactionManager as _,
};
use poprako_orchestra::nucl::Error as NuclError;
use poprako_orchestra::{Level, Nucl};
use tracing::instrument;

use poprako_rdb_core::RdbCore;

use crate::part::nucl::{ReptRead, Serial};
use crate::result::BaseError;
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// Selects one of the transaction coordinators owned by the application.
pub struct HybNucl<NR, NS>(NR, NS);

impl<NR, NS> HybNucl<NR, NS> {
    /// Combines the repeatable-read and serializable coordinators.
    pub const fn new(rept_read: NR, serial: NS) -> Self {
        Self(rept_read, serial)
    }

    /// Returns the repeatable-read transaction coordinator.
    pub const fn rept_read(&self) -> &NR {
        &self.0
    }

    /// Returns the serializable transaction coordinator.
    pub const fn serial(&self) -> &NS {
        &self.1
    }
}

define_sql_function! {
    /// Reads a `PostgreSQL` setting through a typed expression.
    fn current_setting(setting_name: Text) -> Text;
}

define_sql_function! {
    /// Updates a `PostgreSQL` setting using bound values, not SQL fragments.
    fn set_config(setting_name: Text, new_value: Text, is_local: Bool) -> Text;
}

// Selects the session default used by Diesel's transaction manager for BEGIN.
trait RdbLevel: Level + Sized {
    /// `PostgreSQL` isolation value for the next transaction.
    const ISOLATION: &'static str;
}

impl RdbLevel for ReptRead {
    // Preserve repeatable-read semantics for this coordinator.
    const ISOLATION: &'static str = "repeatable read";
}

impl RdbLevel for Serial {
    // Preserve serializable semantics for this coordinator.
    const ISOLATION: &'static str = "serializable";
}

// Prevent a cancelled setup or transaction from returning altered session state
// to the pool. Disarm only after transaction completion and setting restoration.
struct TransactionSession<L> {
    // Connection and its Orchestra isolation marker.
    context: RdbContext<L>,
    // Whether the original session settings have been restored.
    restored: bool,
}

impl<L> Drop for TransactionSession<L> {
    // The pool discards connections whose transaction manager is broken.
    fn drop(&mut self) {
        //
        if self.restored {
            return;
        }

        *AnsiTransactionManager::transaction_manager_status_mut(
            self.context.conn(),
        ) = TransactionManagerStatus::InError;
    }
}

/// Diesel-backed transaction coordinator that wraps operations in database transactions.
///
/// Each call to [`Nucl::coord`] opens a new connection, begins a transaction,
/// runs the closure, and commits or rolls back on success or failure.
pub struct RdbNucl<L = ReptRead> {
    /// Shared database connection pool used for transactions.
    core: RdbCore,
    /// Isolation-level marker carried by this coordinator.
    level: PhantomData<L>,
}

impl<L> RdbNucl<L> {
    /// Builds a new `RdbNucl` from an [`RdbCore`] connection pool.
    #[must_use]
    pub const fn new(core: RdbCore) -> Self {
        Self {
            core,
            level: PhantomData,
        }
    }
}

impl<L> Clone for RdbNucl<L> {
    // Share the existing database pool without constraining the level marker.
    fn clone(&self) -> Self {
        Self::new(self.core.clone())
    }
}

impl<L> Nucl for RdbNucl<L>
where
    L: RdbLevel + Send + Sync,
{
    // Exposes the coordinator's isolation level.
    type Level = L;

    // Transaction error type propagated through the Nucl trait.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Transaction context wrapping a pooled Diesel connection.
    type Context = RdbContext<L>;

    // Coordinates a closure within a database transaction, committing on success
    // and rolling back on error.
    #[instrument(level = "info", skip_all)]
    async fn coord<F, T, E>(&self, f: F) -> Result<T, NuclError<Self::Error, E>>
    where
        F: AsyncFnOnce(&mut Self::Context) -> Result<T, E> + Send,
        T: Send,
        E: Send,
    {
        let conn = self
            .core
            .get()
            .await
            .map_err(|source| NuclError::Backend(source.into()))?;

        let mut session = TransactionSession {
            context: RdbContext::new(conn),
            restored: false,
        };

        let context = &mut session.context;

        let original_isolation =
            diesel::select(current_setting("default_transaction_isolation"))
                .get_result::<String>(context.conn())
                .await
                .map_err(|error| NuclError::Backend(diesel(error)))?;

        diesel::select(set_config(
            "default_transaction_isolation",
            L::ISOLATION,
            false,
        ))
        .get_result::<String>(context.conn())
        .await
        .map_err(|error| NuclError::Backend(diesel(error)))?;

        AnsiTransactionManager::begin_transaction(context.conn())
            .await
            .map_err(|error| NuclError::Backend(diesel(error)))?;

        let result = match f(context).await {
            //
            Ok(value) => {
                //
                AnsiTransactionManager::commit_transaction(context.conn())
                    .await
                    .map_err(|error| NuclError::Backend(diesel(error)))?;

                Ok(value)
            }

            Err(error) => {
                //
                AnsiTransactionManager::rollback_transaction(context.conn())
                    .await
                    .map_err(|rollback_error| {
                        NuclError::Backend(diesel(rollback_error))
                    })?;

                Err(NuclError::Step(error))
            }
        };

        let restoration = diesel::select(set_config(
            "default_transaction_isolation",
            original_isolation,
            false,
        ))
        .get_result::<String>(context.conn())
        .await;

        // A cleanup failure must not turn a committed business result into an
        // apparent transaction failure. The armed guard discards this session.
        if let Err(error) = restoration {
            //
            let _ = diesel(error);

            return result;
        }

        session.restored = true;

        result
    }
}
