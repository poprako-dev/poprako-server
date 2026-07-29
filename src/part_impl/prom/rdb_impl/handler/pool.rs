//! Fixed-size worker pool for persisted prom tasks.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use poprako_orchestra::OperStep as _;
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
use crate::result::BaseRest;

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `tests` module.
mod tests;

// Constant definition for `WORKER_COUNT`.
const WORKER_COUNT: usize = 4;
// Constant definition for `POLL_INTERVAL`.
const POLL_INTERVAL: StdDuration = StdDuration::from_secs(60);
// Constant definition for `STUCK_RESET_INTERVAL`.
const STUCK_RESET_INTERVAL: Duration = Duration::minutes(1);
// Constant definition for `RETRY_DELAY`.
const RETRY_DELAY: Duration = Duration::minutes(5);
// Constant definition for `PROCESSING_TIMEOUT`.
const PROCESSING_TIMEOUT: Duration = Duration::minutes(15);
// Constant definition for `COMPLETED_RETENTION`.
const COMPLETED_RETENTION: Duration = Duration::days(7);
// Constant definition for `DEAD_RETENTION`.
const DEAD_RETENTION: Duration = Duration::days(30);
// Constant definition for `COMPLETED_PURGE_INTERVAL`.
const COMPLETED_PURGE_INTERVAL: Duration = Duration::hours(1);

// Constant definition for `FNV_OFFSET_BASIS`.
const FNV_OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
// Constant definition for `FNV_PRIME`.
const FNV_PRIME: u64 = 1_099_511_628_211;

// Internal type alias for `WorkerSender`.
type WorkerSender = mpsc::UnboundedSender<LocalMessageRow>;

/// Enforces the retry limit for a task flow.
///
/// When the task has been retried 3 or more times, transitions from
/// [`TaskFlow::Retry`] to [`TaskFlow::Dead`] so the message is not
/// requeued indefinitely.
pub fn enforce_retry_limit(
    task_flow: TaskFlow,
    retried_count: i64,
) -> TaskFlow {
    match (task_flow, retried_count >= 3) {
        //
        // Internal implementation detail.
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
        // Internal implementation detail.
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

    // Internal implementation of `spawn_workers`.
    fn spawn_workers(
        self: &Arc<Self>,
        completed: Arc<Notify>,
    ) -> (Vec<WorkerSender>, Vec<JoinHandle<()>>) {
        //
        // Internal implementation detail.
        let mut worker_senders = Vec::with_capacity(WORKER_COUNT);

        let mut worker_handles = Vec::with_capacity(WORKER_COUNT);

        for worker_index in 0..WORKER_COUNT {
            //
            // Internal implementation detail.
            let (worker_sender, mut worker_receiver) =
                mpsc::unbounded_channel();

            let handler = self.clone();

            let completed = completed.clone();

            let worker_handle = tokio::spawn(async move {
                //
                // Internal implementation detail.
                while let Some(row) = worker_receiver.recv().await {
                    //
                    // Internal implementation detail.
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

    // Internal implementation of `run_supervisor`.
    async fn run_supervisor(
        &self,
        worker_senders: &[WorkerSender],
        completed: &Notify,
    ) {
        //
        // Internal implementation detail.
        let mut next_stuck_reset_at = OffsetDateTime::now_utc();

        let mut next_completed_purge_at = OffsetDateTime::now_utc();

        loop {
            //
            // Internal implementation detail.
            if self.token.is_cancelled() {
                break;
            }

            let now = OffsetDateTime::now_utc();

            if now >= next_stuck_reset_at {
                //
                // Internal implementation detail.
                self.log_reset_stuck().await;

                next_stuck_reset_at = now + STUCK_RESET_INTERVAL;
            }

            if now >= next_completed_purge_at {
                //
                // Internal implementation detail.
                self.log_purge_completed().await;

                next_completed_purge_at = now + COMPLETED_PURGE_INTERVAL;
            }

            let dispatched = match self.poll().await {
                //
                // Internal implementation detail.
                Ok(rows) => self.dispatch_rows(worker_senders, rows).await,

                Err(error) => {
                    //
                    // Internal implementation detail.
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

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `process_row`.
    async fn process_row(&self, row: &LocalMessageRow) {
        //
        // Internal implementation detail.
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
            // Internal implementation detail.
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
                // Internal implementation detail.
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

    // Internal implementation of `log_reset_stuck`.
    async fn log_reset_stuck(&self) {
        if let Err(error) = self.reset_stuck().await {
            tracing::error!(
                error = ?error,
                "[RdbPromHandler::run] reset stuck failed",
            );
        }
    }

    // Internal implementation of `log_purge_completed`.
    async fn log_purge_completed(&self) {
        match self.purge_completed().await {
            //
            // Internal implementation detail.
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
    // Internal implementation of `poll`.
    async fn poll(&self) -> BaseRest<Vec<LocalMessageRow>> {
        //
        // Internal implementation detail.
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        PollPending.step_on(&self.repo, &mut context).await
    }

    // Internal implementation of `dispatch_rows`.
    async fn dispatch_rows(
        &self,
        worker_senders: &[WorkerSender],
        rows: Vec<LocalMessageRow>,
    ) -> bool {
        //
        // Internal implementation detail.
        let mut dispatched = false;

        for row in rows {
            //
            // Internal implementation detail.
            let claimed = match self.claim(&row.f_id, row.f_lease).await {
                //
                // Internal implementation detail.
                Ok(claimed) => claimed,

                Err(error) => {
                    //
                    // Internal implementation detail.
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
                // Internal implementation detail.
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
    // Internal implementation of `complete`.
    async fn complete(&self, id: &str, lease: i64) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        CompleteMessage::new(id, lease)
            .step_on(&self.repo, &mut context)
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `retry`.
    async fn retry(&self, id: &str, lease: i64, message: &str) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let visible_at = OffsetDateTime::now_utc() + RETRY_DELAY;

        RetryMessage::new(id, lease, message, &visible_at)
            .step_on(&self.repo, &mut context)
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `fail`.
    async fn fail(&self, id: &str, lease: i64, message: &str) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        FailMessage::new(id, lease, message)
            .step_on(&self.repo, &mut context)
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `reset_stuck`.
    async fn reset_stuck(&self) -> BaseRest<()> {
        //
        // Internal implementation detail.
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let before = OffsetDateTime::now_utc() - PROCESSING_TIMEOUT;

        ResetStuck::new(&before)
            .step_on(&self.repo, &mut context)
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `purge_completed`.
    async fn purge_completed(&self) -> BaseRest<usize> {
        //
        // Internal implementation detail.
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let completed_before = OffsetDateTime::now_utc() - COMPLETED_RETENTION;

        let dead_before = OffsetDateTime::now_utc() - DEAD_RETENTION;

        PurgeCompleted::new(&completed_before, &dead_before)
            .step_on(&self.repo, &mut context)
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `claim`.
    async fn claim(&self, id: &str, lease: i64) -> BaseRest<bool> {
        //
        // Internal implementation detail.
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        ClaimPending::new(id, lease)
            .step_on(&self.repo, &mut context)
            .await
    }
}

// Internal implementation of `topic_worker_index`.
fn topic_worker_index(topic: &str) -> usize {
    //
    // Internal implementation detail.
    let hash = topic.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    });

    hash as usize % WORKER_COUNT
}
