//! Fixed-size worker pool for persisted prom tasks.

#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
// Internal organization of the `tests` module.
mod tests;

use std::sync::Arc;
use std::time::Duration as StdDuration;

use poprako_orchestra::{Nucl as _, OperStep as _};
use time::{Duration, OffsetDateTime};
use tokio::sync::{Notify, OwnedSemaphorePermit};
use tokio::task::JoinSet;
use tokio::time::{Instant, sleep, timeout};
use tracing::instrument;
use uuid::Uuid;

use poprako_obj_dept::ObjDeptView;

use crate::part::effect::Develop;
use crate::part::obj_dept::PageImage;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::actor::base::{
    RdbPromActor, WorkerSlot, run_worker, shutdown_workers,
};
use crate::part_impl::prom::rdb_impl::entity::LocalMessageRow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, CompleteMessage, FailMessage, PurgeDead, ResetStuck,
    RetryMessage,
};
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

// Constant definition for `WORKER_COUNT`.
const WORKER_COUNT: usize = 4;

// Constant definition for `POLL_INTERVAL`.
const POLL_INTERVAL: StdDuration = StdDuration::from_mins(1);

// Constant definition for `STUCK_RESET_INTERVAL`.
const STUCK_RESET_INTERVAL: Duration = Duration::minutes(1);

// Constant definition for `RETRY_DELAY`.
const RETRY_DELAY: Duration = Duration::minutes(5);

// Constant definition for `PROCESSING_TIMEOUT`.
const PROCESSING_TIMEOUT: Duration = Duration::minutes(15);

// Constant definition for `DEAD_RETENTION`.
const DEAD_RETENTION: Duration = Duration::days(30);

// Constant definition for `DEAD_PURGE_INTERVAL`.
const DEAD_PURGE_INTERVAL: Duration = Duration::hours(1);

// Leaves a safety margin before the database reclaims processing attempts.
const EXECUTION_TIMEOUT: StdDuration = StdDuration::from_mins(14);

// Bounds graceful shutdown across the entire pool.
const SHUTDOWN_GRACE: StdDuration = StdDuration::from_secs(30);

/// Enforces the retry limit for a task flow.
///
/// When the task has been retried 3 or more times, transitions from
/// [`TaskFlow::Retry`] to [`TaskFlow::Dead`] so the message is not
/// requeued indefinitely.
pub fn enforce_retry_limit(
    task_flow: TaskFlow,
    retried_count: i64,
) -> TaskFlow {
    //
    match (task_flow, retried_count >= 3) {
        //
        (TaskFlow::Retry { err_message: error }, true) => {
            TaskFlow::Dead { err_message: error }
        }

        (task_flow, _) => task_flow,
    }
}

impl<R, V, D> RdbPromActor<RdbNucl, R, V, D>
where
    R: AssignmentInvitationRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + PageRepo<RdbContext>
        + Send
        + Sync
        + 'static,
    V: ObjDeptView<PageImage, RdbContext> + Send + Sync + 'static,
    D: Develop + Send + Sync + 'static,
{
    /// Runs polling with bounded attempts and a shared worker shutdown deadline.
    #[instrument(level = "info", skip_all)]
    pub async fn run(self) {
        //
        let (actor, completed) = (Arc::new(self), Arc::new(Notify::new()));

        let (worker_sends, worker_handles) = actor.spawn_workers(&completed);

        tokio::select! {
            //
            () = actor.token().cancelled() => {}

            () = actor.run_supervisor(&worker_sends, completed.as_ref()) => {}
        }

        drop(worker_sends);

        shutdown_workers(worker_handles, SHUTDOWN_GRACE).await;
    }

    // Internal implementation of `spawn_workers`.
    fn spawn_workers(
        self: &Arc<Self>,
        completed: &Arc<Notify>,
    ) -> (Vec<WorkerSlot<LocalMessageRow>>, JoinSet<()>) {
        //
        let (mut worker_sends, mut worker_handles) =
            (Vec::with_capacity(WORKER_COUNT), JoinSet::new());

        for _ in 0..WORKER_COUNT {
            //
            let (worker_send, worker_recv) = WorkerSlot::channel();

            let actor = self.clone();

            worker_handles.spawn(run_worker(
                worker_recv,
                completed.clone(),
                move |row| {
                    //
                    let actor = actor.clone();

                    async move { actor.process_row(&row).await }
                },
            ));

            worker_sends.push(worker_send);
        }

        (worker_sends, worker_handles)
    }

    // Internal implementation of `process_row`.
    #[instrument(level = "info", skip_all)]
    async fn process_row(&self, row: &LocalMessageRow) {
        //
        let task_flow =
            self.dispatch_payload(&row.f_topic, &row.f_payload).await;

        let task_flow = enforce_retry_limit(task_flow, row.f_retried_count);

        let task_flow =
            task_flow.limit_wait(row.f_created_at, OffsetDateTime::now_utc());

        match task_flow {
            //
            TaskFlow::Complete => {
                //
                if let Err(error) =
                    self.complete(&row.f_id, row.f_claim_token).await
                {
                    tracing::error!(
                        id = %row.f_id,
                        err = ?error,
                        "[RdbPromActor::process_row] complete failed",
                    );
                }
            }

            TaskFlow::Retry { err_message: error } => {
                self.log_reschedule(row, &error, 1).await;
            }

            TaskFlow::Wait { err_message: error } => {
                self.log_reschedule(row, &error, 0).await;
            }

            TaskFlow::Dead { err_message: error } => {
                //
                tracing::error!(
                    id = %row.f_id,
                    topic = %row.f_topic,
                    err = %error,
                    "[RdbPromActor::process_row] task failed",
                );

                if let Err(mark_error) =
                    self.fail(&row.f_id, row.f_claim_token, &error).await
                {
                    tracing::error!(
                        id = %row.f_id,
                        original_err = %error,
                        err = ?mark_error,
                        "[RdbPromActor::process_row] fail mark failed",
                    );
                }
            }
        }
    }

    // Internal implementation of `complete`.
    #[instrument(level = "info", skip_all)]
    async fn complete(&self, id: &str, claim_token: Uuid) -> BaseRest<()> {
        //
        self.prom_nucl()
            .coord(async |context| {
                //
                CompleteMessage::new(id, claim_token)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(())
    }

    // Logs a failed attempt to return one task to pending.
    async fn log_reschedule(
        &self,
        row: &LocalMessageRow,
        message: &str,
        retry_delta: i64,
    ) {
        //
        let rest = async {
            //
            let visible_at = OffsetDateTime::now_utc()
                .checked_add(RETRY_DELAY)
                .ok_or_else(|| BaseError::Unrecoverable {
                    message:
                        "prom retry timestamp is outside the supported range"
                            .into(),
                })?;

            self.prom_nucl()
                .coord(async |context| {
                    //
                    RetryMessage::new(
                        &row.f_id,
                        row.f_claim_token,
                        message,
                        &visible_at,
                        retry_delta,
                    )
                    .step_on(self.prom_repo(), context)
                    .await
                })
                .await?;

            Ok::<(), BaseError>(())
        }
        .await;

        if let Err(mark_error) = rest {
            //
            tracing::error!(
                id = %row.f_id,
                original_err = %message,
                err = ?mark_error,
                "[RdbPromActor::process_row] reschedule failed",
            );
        }
    }

    // Internal implementation of `fail`.
    #[instrument(level = "info", skip_all)]
    async fn fail(
        &self,
        id: &str,
        claim_token: Uuid,
        message: &str,
    ) -> BaseRest<()> {
        //
        self.prom_nucl()
            .coord(async |context| {
                //
                FailMessage::new(id, claim_token, message)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(())
    }

    // Internal implementation of `reset_stuck`.
    #[instrument(level = "info", skip_all)]
    async fn reset_stuck(&self) -> BaseRest<()> {
        //
        let before = OffsetDateTime::now_utc()
            .checked_sub(PROCESSING_TIMEOUT)
            .ok_or_else(|| BaseError::Unrecoverable {
                message:
                    "prom processing cutoff is outside the supported range"
                        .into(),
            })?;

        self.prom_nucl()
            .coord(async |context| {
                //
                ResetStuck::new(&before)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(())
    }

    // Internal implementation of `purge_dead`.
    #[instrument(level = "info", skip_all)]
    async fn purge_dead(&self) -> BaseRest<usize> {
        //
        let dead_before = OffsetDateTime::now_utc()
            .checked_sub(DEAD_RETENTION)
            .ok_or_else(|| BaseError::Unrecoverable {
                message:
                    "prom dead-message cutoff is outside the supported range"
                        .into(),
            })?;

        let purged_count = self
            .prom_nucl()
            .coord(async |context| {
                //
                PurgeDead::new(&dead_before)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(purged_count)
    }

    // Internal implementation of `log_reset_stuck`.
    async fn log_reset_stuck(&self) {
        //
        if let Err(error) = self.reset_stuck().await {
            //
            tracing::error!(
                err = ?error,
                "[RdbPromActor::run] reset stuck failed",
            );
        }
    }

    // Internal implementation of `log_purge_dead`.
    async fn log_purge_dead(&self) {
        //
        match self.purge_dead().await {
            //
            Ok(purged_count) => {
                //
                if purged_count > 0 {
                    //
                    tracing::info!(
                        purged_count,
                        "[RdbPromActor::run] purged expired dead messages",
                    );
                }
            }

            Err(error) => {
                //
                tracing::error!(
                    err = ?error,
                    "[RdbPromActor::run] purge dead failed",
                );
            }
        }
    }

    // Internal implementation of `poll`.
    #[instrument(level = "info", skip_all)]
    async fn poll(&self, limit: usize) -> BaseRest<Vec<LocalMessageRow>> {
        //
        let rows = self
            .prom_nucl()
            .coord(async |context| {
                //
                ClaimPending::new(limit)
                    .step_on(self.prom_repo(), context)
                    .await
            })
            .await?;

        Ok(rows)
    }

    // Assigns claimed tasks only to workers reserved before the claim.
    fn dispatch_local_messages(
        reservations: Vec<(&WorkerSlot<LocalMessageRow>, OwnedSemaphorePermit)>,
        rows: Vec<LocalMessageRow>,
        deadline: Instant,
    ) -> bool {
        //
        let mut dispatched = false;

        for ((worker, permit), row) in reservations.into_iter().zip(rows) {
            //
            if worker.dispatch(row, deadline, permit) {
                //
                dispatched = true;

                continue;
            }

            tracing::error!("prom reserved worker channel closed");
        }

        dispatched
    }

    // Internal implementation of `run_supervisor`.
    async fn run_supervisor(
        &self,
        worker_sends: &[WorkerSlot<LocalMessageRow>],
        completed: &Notify,
    ) {
        //
        let (mut next_stuck_reset_at, mut next_dead_purge_at) =
            (OffsetDateTime::now_utc(), OffsetDateTime::now_utc());

        loop {
            //
            if self.token().is_cancelled() {
                break;
            }

            let maintenance_and_poll = async {
                //
                let now = OffsetDateTime::now_utc();

                if now >= next_stuck_reset_at {
                    //
                    self.log_reset_stuck().await;

                    next_stuck_reset_at =
                        schedule_at(now, STUCK_RESET_INTERVAL);
                }

                if now >= next_dead_purge_at {
                    //
                    self.log_purge_dead().await;

                    next_dead_purge_at = schedule_at(now, DEAD_PURGE_INTERVAL);
                }

                let reservations = worker_sends
                    .iter()
                    .filter_map(|worker| {
                        worker.acquire().map(|permit| (worker, permit))
                    })
                    .collect::<Vec<_>>();

                if reservations.is_empty() {
                    return false;
                }

                let deadline = Instant::now() + EXECUTION_TIMEOUT;

                match self.poll(reservations.len()).await {
                    //
                    Ok(local_messages) => Self::dispatch_local_messages(
                        reservations,
                        local_messages,
                        deadline,
                    ),

                    Err(error) => {
                        //
                        tracing::error!(err = ?error, "prom claim failed");

                        false
                    }
                }
            };

            let dispatched = timeout(POLL_INTERVAL, maintenance_and_poll)
                .await
                .unwrap_or_else(|_| {
                    //
                    tracing::warn!(
                        "prom queue maintenance or polling timed out"
                    );

                    false
                });

            if dispatched {
                continue;
            }

            tokio::select! {
                //
                biased;

                () = self.token().cancelled() => break,

                () = completed.notified() => {}

                () = sleep(POLL_INTERVAL) => {}
            }
        }
    }
}

// Computes the next maintenance deadline without panicking at time bounds.
const fn schedule_at(
    now: OffsetDateTime,
    interval: Duration,
) -> OffsetDateTime {
    now.saturating_add(interval)
}
