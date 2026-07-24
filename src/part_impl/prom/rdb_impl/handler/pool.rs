//! Fixed-size worker pool for persisted prom tasks.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use poprako_orchestra::Step as _;
use time::{Duration, OffsetDateTime};
use tokio::sync::{Notify, mpsc};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::instrument;

use crate::part::effect::EffectDevelop;
use crate::part::image::ImageManager;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageRow;
use crate::part_impl::prom::rdb_impl::handler::base::{
    RdbPromHandler, dispatch_payload,
};
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, CompleteMessage, FailMessage, PollPending, PurgeCompleted,
    ResetStuck, RetryMessage,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::shared::RdbContext;
use crate::result::BaseResult;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
mod tests;

const WORKER_COUNT: usize = 4;
const POLL_INTERVAL: StdDuration = StdDuration::from_secs(60);
const STUCK_RESET_INTERVAL: Duration = Duration::minutes(1);
const RETRY_DELAY: Duration = Duration::minutes(5);
const PROCESSING_TIMEOUT: Duration = Duration::minutes(15);
const COMPLETED_RETENTION: Duration = Duration::days(7);
const DEAD_RETENTION: Duration = Duration::days(30);
const COMPLETED_PURGE_INTERVAL: Duration = Duration::hours(1);

const FNV_OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

type WorkerSender = mpsc::UnboundedSender<LocalMessageRow>;

/// Enforces the retry limit for a task flow.
///
/// When the task has been retried 3 or more times, transitions from
/// [`TaskFlow::Retry`] to [`TaskFlow::Dead`] so the message is not
/// requeued indefinitely.
pub(super) fn enforce_retry_limit(
    task_flow: TaskFlow,
    retried_count: i64,
) -> TaskFlow {
    match (task_flow, retried_count >= 3) {
        //
        (TaskFlow::Retry(error), true) => TaskFlow::Dead(error),

        (task_flow, _) => task_flow,
    }
}

impl<I, V> RdbPromHandler<RdbDrive, RdbRepo, I, V>
where
    I: ImageManager + Send + Sync + 'static,
    V: EffectDevelop + Send + Sync + 'static,
{
    /// Runs the polling supervisor and drains in-flight worker tasks on shutdown.
    #[instrument(level = "info", skip_all)]
    pub async fn run(self) {
        //
        let handler = Arc::new(self);

        let completed = Arc::new(Notify::new());

        let (worker_senders, worker_handles) =
            handler.spawn_workers(completed.clone());

        handler
            .run_supervisor(&worker_senders, completed.as_ref())
            .await;

        drop(worker_senders);

        for worker_handle in worker_handles {
            if let Err(error) = worker_handle.await {
                tracing::error!(
                    error = ?error,
                    "[RdbPromHandler::run] worker task failed",
                );
            }
        }
    }

    fn spawn_workers(
        self: &Arc<Self>,
        completed: Arc<Notify>,
    ) -> (Vec<WorkerSender>, Vec<JoinHandle<()>>) {
        //
        let mut worker_senders = Vec::with_capacity(WORKER_COUNT);

        let mut worker_handles = Vec::with_capacity(WORKER_COUNT);

        for worker_index in 0..WORKER_COUNT {
            //
            let (worker_sender, mut worker_receiver) =
                mpsc::unbounded_channel();

            let handler = self.clone();

            let completed = completed.clone();

            let worker_handle = tokio::spawn(async move {
                //
                while let Some(row) = worker_receiver.recv().await {
                    //
                    handler.process_row(&row).await;

                    completed.notify_one();
                }

                tracing::debug!(
                    worker_index,
                    "[RdbPromHandler::worker] worker stopped",
                );
            });

            worker_senders.push(worker_sender);

            worker_handles.push(worker_handle);
        }

        (worker_senders, worker_handles)
    }

    async fn run_supervisor(
        &self,
        worker_senders: &[WorkerSender],
        completed: &Notify,
    ) {
        //
        let mut next_stuck_reset_at = OffsetDateTime::now_utc();

        let mut next_completed_purge_at = OffsetDateTime::now_utc();

        loop {
            //
            if self.token.is_cancelled() {
                break;
            }

            let now = OffsetDateTime::now_utc();

            if now >= next_stuck_reset_at {
                //
                self.log_reset_stuck().await;

                next_stuck_reset_at = now + STUCK_RESET_INTERVAL;
            }

            if now >= next_completed_purge_at {
                //
                self.log_purge_completed().await;

                next_completed_purge_at = now + COMPLETED_PURGE_INTERVAL;
            }

            let dispatched = match self.poll().await {
                //
                Ok(rows) => self.dispatch_rows(worker_senders, rows).await,

                Err(error) => {
                    //
                    tracing::error!(
                        error = ?error,
                        "[RdbPromHandler::run] poll failed",
                    );

                    false
                }
            };

            if dispatched {
                continue;
            }

            tokio::select! {
                biased;
                () = self.token.cancelled() => break,
                () = completed.notified() => {}
                _ = sleep(POLL_INTERVAL) => {}
            }
        }
    }

    async fn dispatch_rows(
        &self,
        worker_senders: &[WorkerSender],
        rows: Vec<LocalMessageRow>,
    ) -> bool {
        //
        let mut dispatched = false;

        for row in rows {
            //
            let claimed = match self.claim(&row.f_id, row.f_lease).await {
                //
                Ok(claimed) => claimed,

                Err(error) => {
                    //
                    tracing::error!(
                        id = %row.f_id,
                        error = ?error,
                        "[RdbPromHandler::dispatch_rows] claim failed",
                    );

                    continue;
                }
            };

            if !claimed {
                continue;
            }

            let worker_index = topic_worker_index(&row.f_topic);

            match worker_senders[worker_index].send(row) {
                //
                Ok(()) => dispatched = true,

                Err(error) => {
                    tracing::error!(
                        id = %error.0.f_id,
                        worker_index,
                        "[RdbPromHandler::dispatch_rows] worker channel closed",
                    );
                }
            }
        }

        dispatched
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn poll(&self) -> BaseResult<Vec<LocalMessageRow>> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo.step(&mut context, &PollPending).await
    }

    #[instrument(level = "info", skip_all)]
    async fn process_row(&self, row: &LocalMessageRow) {
        //
        let task_flow = dispatch_payload(
            &self.nucl,
            self.repo.inner(),
            &self.image_pool,
            &self.develop,
            &row.f_topic,
            &row.f_payload,
        )
        .await;

        let task_flow = enforce_retry_limit(task_flow, row.f_retried_count);

        match task_flow {
            //
            TaskFlow::Complete => {
                if let Err(error) = self.complete(&row.f_id, row.f_lease).await
                {
                    tracing::error!(
                        id = %row.f_id,
                        error = ?error,
                        "[RdbPromHandler::process_row] complete failed",
                    );
                }
            }

            TaskFlow::Retry(error) => {
                if let Err(mark_error) =
                    self.retry(&row.f_id, row.f_lease, &error).await
                {
                    tracing::error!(
                        id = %row.f_id,
                        original_error = %error,
                        error = ?mark_error,
                        "[RdbPromHandler::process_row] retry mark failed",
                    );
                }
            }

            TaskFlow::Dead(error) => {
                //
                tracing::error!(
                    id = %row.f_id,
                    topic = %row.f_topic,
                    error = %error,
                    "[RdbPromHandler::process_row] task failed",
                );

                if let Err(mark_error) =
                    self.fail(&row.f_id, row.f_lease, &error).await
                {
                    tracing::error!(
                        id = %row.f_id,
                        original_error = %error,
                        error = ?mark_error,
                        "[RdbPromHandler::process_row] fail mark failed",
                    );
                }
            }
        }
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn claim(&self, id: &str, lease: i64) -> BaseResult<bool> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo
            .step(&mut context, &ClaimPending::new(id, lease))
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn complete(&self, id: &str, lease: i64) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo
            .step(&mut context, &CompleteMessage::new(id, lease))
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn fail(
        &self,
        id: &str,
        lease: i64,
        message: &str,
    ) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo
            .step(&mut context, &FailMessage::new(id, lease, message))
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn retry(
        &self,
        id: &str,
        lease: i64,
        message: &str,
    ) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let visible_at = OffsetDateTime::now_utc() + RETRY_DELAY;

        self.repo
            .step(
                &mut context,
                &RetryMessage::new(id, lease, message, &visible_at),
            )
            .await
    }

    async fn log_reset_stuck(&self) {
        if let Err(error) = self.reset_stuck().await {
            tracing::error!(
                error = ?error,
                "[RdbPromHandler::run] reset stuck failed",
            );
        }
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn reset_stuck(&self) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let before = OffsetDateTime::now_utc() - PROCESSING_TIMEOUT;

        self.repo
            .step(&mut context, &ResetStuck::new(&before))
            .await
    }

    async fn log_purge_completed(&self) {
        match self.purge_completed().await {
            //
            Ok(purged_count) => {
                if purged_count > 0 {
                    tracing::info!(
                        purged_count,
                        "[RdbPromHandler::run] purged expired completed messages",
                    );
                }
            }

            Err(error) => {
                tracing::error!(
                    error = ?error,
                    "[RdbPromHandler::run] purge completed failed",
                );
            }
        }
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn purge_completed(&self) -> BaseResult<usize> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let completed_before = OffsetDateTime::now_utc() - COMPLETED_RETENTION;

        let dead_before = OffsetDateTime::now_utc() - DEAD_RETENTION;

        self.repo
            .step(
                &mut context,
                &PurgeCompleted::new(&completed_before, &dead_before),
            )
            .await
    }
}

fn topic_worker_index(topic: &str) -> usize {
    //
    let hash = topic.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    });

    hash as usize % WORKER_COUNT
}
