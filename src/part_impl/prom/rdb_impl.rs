//! Diesel-backed prom (promise) adapter.
//!
//! [`RdbProm`] is both the transactional handle for enqueuing deferred actions
//! into `t_local_message` and the owner of the
//! background consumer task that polls, dispatches, and completes those
//! records — mirroring the self-contained lifecycle of [`AsyncEffectDevelop`].
//!
//! [`AsyncEffectDevelop`]: crate::part_impl::effect::async_impl::AsyncEffectDevelop

// Internal organization of the `entity` module.
mod entity;
// Internal organization of the `actor` module.
mod actor;
// Internal organization of the `repo` module.
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
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::effect::Develop;
use crate::part::image::ImageManager;
use crate::part::nucl::ReptRead;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntryRow;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbContext, RdbCore};

// ── Handle type ────────────────────────────────────────────────────────────

/// RDBMS-backed prom adapter for enqueuing and consuming deferred actions.
///
/// Implements [`Prom<C>`] for transactional task deferral.
/// The constructor spawns a background worker that polls `t_local_message` and
/// dispatches completed records by topic.
///
/// Call [`close`](RdbProm::close) before dropping to finish in-flight work
/// gracefully. Pending records remain durable for the next worker start.
pub struct RdbProm {
    //
    // Internal state field `token`.
    /// Cancellation token to signal graceful shutdown of the prom processor.
    token: CancellationToken,
    /// Watch receiver that signals when background processing drains.
    done: watch::Receiver<bool>,
}

impl RdbProm {
    /// Creates the prom adapter and launches its background consumer task.
    ///
    /// The supervisor polls `t_local_message` and routes each topic to one of four
    /// serial worker tasks. Different topics can run concurrently, while messages
    /// from one topic never execute concurrently in this process.
    pub fn new<I, D>(core: RdbCore, image_pool: I, develop: D) -> Self
    where
        I: ImageManager + Send + Sync + 'static,
        D: Develop + Send + Sync + 'static,
    {
        let token = CancellationToken::new();

        let (done_send, done) = watch::channel(false);

        let (nucl, repo) = (
            RdbNucl::new(core.clone()),
            RdbPromRepo::new(HybRepo::new(core)),
        );

        let actor = actor::RdbPromActor::new(
            nucl,
            repo,
            image_pool,
            develop,
            token.clone(),
        );

        tokio::spawn(async move {
            //
            // Internal implementation detail.
            actor.run().await;

            done_send.send_replace(true);
        });

        Self { token, done }
    }

    /// Signals the background worker to stop and waits for in-flight work
    /// to complete.
    #[instrument(level = "info", skip_all)]
    pub async fn close(&self) {
        //
        // Internal implementation detail.
        self.token.cancel();

        let mut done = self.done.clone();

        if let Err(error) = done.wait_for(|done| *done).await {
            //
            tracing::error!(
                err = %error,
                "[RdbProm::close] background task ended without completion",
            );
        }
    }
}

impl Drop for RdbProm {
    // Internal implementation of `drop`.
    fn drop(&mut self) {
        self.token.cancel();
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
