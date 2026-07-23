//! Diesel-backed prom (promise) adapter.
//!
//! [`RdbProm`] is both the transactional handle for enqueuing deferred actions
//! into `t_local_message` and the owner of the
//! background consumer task that polls, dispatches, and completes those
//! records — mirroring the self-contained lifecycle of [`AsyncEffectDevelop`].
//!
//! [`AsyncEffectDevelop`]: crate::part_impl::effect::async_impl::AsyncEffectDevelop

use diesel_async::RunQueryDsl;
use poprako_orchestra::Step;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use time::OffsetDateTime;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::image::ImageManager;
use crate::part::prom::Prom;
use crate::part::prom::payload::Payload;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntry;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbContext, RdbCore};
use crate::result::{BaseError, BaseResult, accept};

mod entity;
mod handler;
mod repo;
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod test_shared;
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod tests;

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
    /// Cancellation token to signal graceful shutdown of the prom processor.
    token: CancellationToken,
    /// Watch receiver that signals when background processing drains.
    done: watch::Receiver<bool>,
}

impl RdbProm {
    /// Creates the prom adapter and starts its background consumer task.
    ///
    /// The supervisor polls `t_local_message` and routes each topic to one of four
    /// serial worker tasks. Different topics can run concurrently, while messages
    /// from one topic never execute concurrently in this process.
    pub fn new<I>(core: RdbCore, image_pool: I) -> Self
    where
        I: ImageManager + Send + Sync + 'static,
    {
        let token = CancellationToken::new();

        let (done_send, done) = watch::channel(false);

        let drive = RdbDrive::new(core.clone());

        let repo = RdbPromRepo::new(RdbRepo::new(core.clone()));

        let handler = handler::RdbPromHandler::new(
            core,
            drive,
            repo,
            image_pool,
            token.clone(),
        );

        tokio::spawn(async move {
            //
            handler.run().await;

            done_send.send_replace(true);
        });

        Self { token, done }
    }

    /// Signals the background worker to stop and waits for in-flight work
    /// to complete.
    #[instrument(level = "info", skip_all)]
    pub async fn close(&self) {
        //
        self.token.cancel();

        let mut done = self.done.clone();

        match done.wait_for(|done| *done).await {
            //
            Ok(_) => {}

            Err(error) => {
                tracing::error!(
                    error = %error,
                    "[RdbProm::close] background task ended without completion",
                );
            }
        };
    }
}

impl Drop for RdbProm {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

impl<'a> Step<Defer<'a, String, Payload, ()>, RdbContext> for RdbProm {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &Defer<'a, String, Payload, ()>,
    ) -> BaseResult<()> {
        //
        let now = OffsetDateTime::now_utc();

        let entry = LocalMessageEntry::from_task(&oper.task, now)?;

        diesel::insert_into(t_local_message::table)
            .values(&entry)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        accept(())
    }
}

impl<'t, 'a> Step<DeferBatch<'t, 'a, String, Payload, ()>, RdbContext>
    for RdbProm
{
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeferBatch<'t, 'a, String, Payload, ()>,
    ) -> BaseResult<()> {
        //
        if oper.tasks.is_empty() {
            return accept(());
        }

        let now = OffsetDateTime::now_utc();

        let entries = oper
            .tasks
            .iter()
            .map(|task| LocalMessageEntry::from_task(task, now))
            .collect::<BaseResult<Vec<_>>>()?;

        diesel::insert_into(t_local_message::table)
            .values(&entries)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        accept(())
    }
}

impl Prom<RdbContext> for RdbProm {}
