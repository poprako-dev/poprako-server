//! Diesel-backed prom (promise) adapter.
//!
//! [`RdbProm`] writes deferred actions into `t_local_message` through the
//! caller's transaction context.

// Internal organization of the `entity` module.
#[allow(dead_code)]
mod entity;
// Internal organization of the `actor` module.
#[allow(dead_code)]
mod actor;
// Internal organization of the `repo` module.
#[allow(dead_code)]
mod repo;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `test_shared` module.
mod test_shared;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `tests` module.
mod tests;

use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::part::nucl::ReptRead;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntryRow;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// RDB-backed prom adapter for transactional task deferral.
///
/// Implements [`Prom<C>`] for transactional task deferral.
#[derive(Clone, Copy, Default)]
pub struct RdbProm;

impl RdbProm {
    /// Creates the dependency-free RDB prom adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<'a, L> Step<Defer<'a, String, TaskPayload, ()>, RdbContext<L>> for RdbProm
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &Defer<'a, String, TaskPayload, ()>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let now = OffsetDateTime::now_utc();

        let entry = LocalMessageEntryRow::from_task(&oper.task, now)?;

        diesel::insert_into(t_local_message::table)
            .values(&entry)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        accept(())
    }
}

impl<'t, 'a, L> Step<DeferBatch<'t, 'a, String, TaskPayload, ()>, RdbContext<L>>
    for RdbProm
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeferBatch<'t, 'a, String, TaskPayload, ()>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        if oper.tasks.is_empty() {
            return accept(());
        }

        let now = OffsetDateTime::now_utc();

        let entries = oper
            .tasks
            .iter()
            .map(|task| LocalMessageEntryRow::from_task(task, now))
            .collect::<BaseRest<Vec<_>>>()?;

        diesel::insert_into(t_local_message::table)
            .values(&entries)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        accept(())
    }
}
