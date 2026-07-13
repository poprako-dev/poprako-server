//! Diesel-backed prom (promise) adapter.
//!
//! [`RdbProm`] is both the transactional handle for enqueuing deferred actions
//! into `t_local_message` and the owner of the
//! background consumer task that polls, dispatches, and completes those
//! records — mirroring the self-contained lifecycle of [`AsyncEffectDevelop`].
//!
//! [`AsyncEffectDevelop`]: crate::part_impl::effect::async_impl::AsyncEffectDevelop

mod entity;

mod handler;

mod repo;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use diesel_async::RunQueryDsl;
use poprako_orchestra::Step;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use time::OffsetDateTime;
use tokio::sync::oneshot::{
    Receiver as OneshotReceiver, Sender as OneshotSender,
};

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
use crate::result::{RegularError, RegularResult};

// ── Handle type ────────────────────────────────────────────────────────────

/// RDBMS-backed prom adapter for enqueuing and consuming deferred actions.
///
/// Implements [`Prom<C>`] for transactional task deferral.
/// The constructor spawns a background worker that polls `t_local_message` and
/// dispatches completed records by topic.
///
/// Call [`close`](RdbProm::close) before dropping to drain pending work
/// gracefully.
pub struct RdbProm {
    accepting: Arc<AtomicBool>,
    shutdown: Mutex<Option<OneshotSender<()>>>,
    done: Mutex<Option<OneshotReceiver<()>>>,
}

impl RdbProm {
    /// Creates the prom adapter and starts its background consumer task.
    ///
    /// The worker polls `t_local_message` on a fixed interval, dispatches
    /// records by topic, and updates their lifecycle status.
    pub fn new<I>(core: RdbCore, image_pool: I) -> Self
    where
        I: ImageManager + Send + Sync + 'static,
    {
        let (shutdown_send, shutdown_recv) = tokio::sync::oneshot::channel();

        let (done_send, done_recv) = tokio::sync::oneshot::channel();

        let accepting = Arc::new(AtomicBool::new(true));

        let drive = RdbDrive::new(core.clone());

        let repo = RdbPromRepo::new(RdbRepo::new(core.clone()));

        let handler = handler::RdbPromHandler::new(
            core,
            drive,
            repo,
            image_pool,
            shutdown_recv,
            done_send,
            Arc::clone(&accepting),
        );

        tokio::spawn(async move {
            handler.run().await;
        });

        Self {
            accepting,
            shutdown: Mutex::new(Some(shutdown_send)),
            done: Mutex::new(Some(done_recv)),
        }
    }

    /// Signals the background worker to stop and waits for in-flight work
    /// to complete.
#[instrument(level = "info", skip_all)]
    pub async fn close(&self) {
        //
        if !self.accepting.swap(false, Ordering::AcqRel) {
            return;
        }

        let shutdown_send = self.shutdown.lock().unwrap().take();

        if let Some(shutdown_send) = shutdown_send {
            shutdown_send.send(()).unwrap_or_else(|error| {
                tracing::error!(
                    error = ?error,
                    "[RdbProm::close] background task already terminated",
                );
            });
        }

        let done_recv = self.done.lock().unwrap().take();

        let Some(done_recv) = done_recv else {
            return;
        };

        done_recv.await.unwrap_or_else(|error| {
            tracing::error!(
                error = %error,
                "[RdbProm::close] background task did not signal completion",
            );
        });
    }
}

impl<'a> Step<Defer<'a, String, Payload, ()>, RdbContext> for RdbProm {
    type Error = RegularError;

#[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &Defer<'a, String, Payload, ()>,
    ) -> RegularResult<()> {
        //
        let now = OffsetDateTime::now_utc();

        let entry = LocalMessageEntry::from_task(&oper.task, now)?;

        diesel::insert_into(t_local_message::table)
            .values(&entry)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

impl<'t, 'a> Step<DeferBatch<'t, 'a, String, Payload, ()>, RdbContext>
    for RdbProm
{
    type Error = RegularError;

#[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeferBatch<'t, 'a, String, Payload, ()>,
    ) -> RegularResult<()> {
        //
        if oper.tasks.is_empty() {
            return Ok(());
        }

        let now = OffsetDateTime::now_utc();

        let entries = oper
            .tasks
            .iter()
            .map(|task| LocalMessageEntry::from_task(task, now))
            .collect::<RegularResult<Vec<_>>>()?;

        diesel::insert_into(t_local_message::table)
            .values(&entries)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

impl Prom<RdbContext> for RdbProm {}
