use std::sync::Arc;
use std::time::Duration as StdDuration;

use time::{Duration, OffsetDateTime};
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio::time::{Instant, sleep, timeout};
use tracing::instrument;

use crate::general::actor::PromActor;
use crate::general::actor::worker::{WorkerSlot, run_worker, shutdown_workers};
use crate::general::delivery::{ClaimedTask, Delivery};
use crate::general::dispatch_flow::{DispatchFlow, enforce_retry_limit};
use crate::general::handler::{Handler, Payload};

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

// One topic's reserved execution slot and completion signal.
struct TopicWorker {
    // Topic whose messages are assigned to this worker.
    topic: &'static str,
    // Reserved execution capacity and the worker's sending channel.
    slot: WorkerSlot<ClaimedTask>,
    // Wakes only this topic's polling loop after an attempt finishes.
    completed: Arc<Notify>,
}

impl<P, H> PromActor<P, H>
where
    P: Delivery + Send + Sync + 'static,
    H: Handler + Send + Sync + 'static,
    H::Payload: Send,
{
    /// Runs polling with bounded attempts and a shared worker shutdown deadline.
    #[instrument(level = "info", skip_all)]
    pub async fn run(self) {
        //
        let actor = Arc::new(self);

        let (worker_sends, worker_handles) = actor.spawn_workers();

        let supervisors =
            futures_util::future::join_all(worker_sends.iter().map(|worker| {
                //
                actor.run_supervisor(
                    worker.topic,
                    &worker.slot,
                    worker.completed.as_ref(),
                )
            }));

        tokio::select! {
            //
            () = actor.token.cancelled() => {}

            _ = supervisors => {}
        }

        drop(worker_sends);

        shutdown_workers(worker_handles, SHUTDOWN_GRACE).await;
    }

    // Internal implementation of `spawn_workers`.
    fn spawn_workers(self: &Arc<Self>) -> (Vec<TopicWorker>, JoinSet<()>) {
        //
        let (mut worker_sends, mut worker_handles) =
            (Vec::with_capacity(H::Payload::TOPICS.len()), JoinSet::new());

        for &topic in H::Payload::TOPICS {
            //
            let (worker_send, worker_recv) = WorkerSlot::channel();

            let completed = Arc::new(Notify::new());

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

            let topic_worker = TopicWorker {
                topic,
                slot: worker_send,
                completed,
            };

            worker_sends.push(topic_worker);
        }

        (worker_sends, worker_handles)
    }

    // Internal implementation of `run_supervisor`.
    async fn run_supervisor(
        &self,
        topic: &'static str,
        worker: &WorkerSlot<ClaimedTask>,
        completed: &Notify,
    ) {
        //
        let (mut next_stuck_reset_at, mut next_dead_purge_at) =
            (OffsetDateTime::now_utc(), OffsetDateTime::now_utc());

        loop {
            //
            if self.token.is_cancelled() {
                break;
            }

            let maintenance_and_poll = async {
                //
                let now = OffsetDateTime::now_utc();

                if now >= next_stuck_reset_at {
                    //
                    self.log_reset_stuck(topic).await;

                    next_stuck_reset_at =
                        schedule_at(now, STUCK_RESET_INTERVAL);
                }

                if now >= next_dead_purge_at {
                    //
                    self.log_purge_dead(topic).await;

                    next_dead_purge_at = schedule_at(now, DEAD_PURGE_INTERVAL);
                }

                let Some(permit) = worker.acquire() else {
                    return false;
                };

                let deadline = Instant::now() + EXECUTION_TIMEOUT;

                match self.delivery.claim(topic).await {
                    //
                    Ok(Some(local_task)) => {
                        //
                        if worker.dispatch(local_task, deadline, permit) {
                            return true;
                        }

                        tracing::error!("prom reserved worker channel closed");

                        false
                    }

                    Ok(None) => false,

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
                        "prom delivery maintenance or polling timed out"
                    );

                    false
                });

            if dispatched {
                continue;
            }

            tokio::select! {
                //
                biased;

                () = self.token.cancelled() => break,

                () = completed.notified() => {}

                () = sleep(POLL_INTERVAL) => {}
            }
        }
    }

    // Internal implementation of `process_row`.
    #[instrument(level = "info", skip_all)]
    async fn process_row(&self, row: &ClaimedTask) {
        //
        let dispatch_flow =
            self.dispatch_payload(row.topic(), row.payload()).await;

        let dispatch_flow =
            enforce_retry_limit(dispatch_flow, row.retried_count());

        let dispatch_flow = dispatch_flow
            .limit_wait(row.created_at(), OffsetDateTime::now_utc());

        match dispatch_flow {
            //
            DispatchFlow::Complete => {
                //
                if let Err(error) = self.delivery.complete(row).await {
                    //
                    tracing::error!(
                        id = %row.id(),
                        err = ?error,
                        "[Actor::process_row] complete failed",
                    );
                }
            }

            DispatchFlow::Retry { err_msg: error } => {
                self.log_reschedule(row, &error, 1).await;
            }

            DispatchFlow::Wait { err_msg: error } => {
                self.log_reschedule(row, &error, 0).await;
            }

            DispatchFlow::Dead { err_msg: error } => {
                //
                tracing::error!(
                    id = %row.id(),
                    topic = %row.topic(),
                    err = %error,
                    "[Actor::process_row] task failed",
                );

                if let Err(mark_error) = self.delivery.dead(row, &error).await {
                    //
                    tracing::error!(
                        id = %row.id(),
                        original_err = %error,
                        err = ?mark_error,
                        "[Actor::process_row] fail mark failed",
                    );
                }
            }
        }
    }

    // Internal implementation of `log_reset_stuck`.
    async fn log_reset_stuck(&self, topic: &'static str) {
        //
        let Some(before) =
            OffsetDateTime::now_utc().checked_sub(PROCESSING_TIMEOUT)
        else {
            //
            tracing::error!(
                "prom processing cutoff is outside the supported range"
            );

            return;
        };

        if let Err(error) = self.delivery.reset_stuck(topic, before).await {
            //
            tracing::error!(
                err = ?error,
                "[Actor::run] reset stuck failed",
            );
        }
    }

    // Internal implementation of `log_purge_dead`.
    async fn log_purge_dead(&self, topic: &'static str) {
        //
        let Some(before) =
            OffsetDateTime::now_utc().checked_sub(DEAD_RETENTION)
        else {
            //
            tracing::error!(
                "prom dead-message cutoff is outside the supported range"
            );

            return;
        };

        match self.delivery.purge_dead(topic, before).await {
            //
            Ok(purged_count) => {
                //
                if purged_count > 0 {
                    //
                    tracing::info!(
                        purged_count,
                        "[Actor::run] purged expired dead messages",
                    );
                }
            }

            Err(error) => {
                //
                tracing::error!(
                    err = ?error,
                    "[Actor::run] purge dead failed",
                );
            }
        }
    }

    // Reschedules an attempt without coupling the actor to an error type.
    async fn log_reschedule(
        &self,
        row: &ClaimedTask,
        msg: &str,
        retry_delta: i64,
    ) {
        //
        let Some(visible_at) =
            OffsetDateTime::now_utc().checked_add(RETRY_DELAY)
        else {
            //
            tracing::error!(id = %row.id(), "prom retry timestamp is outside the supported range");

            return;
        };

        if let Err(error) =
            self.delivery.retry(row, msg, visible_at, retry_delta).await
        {
            tracing::error!(id = %row.id(), err = ?error, "prom reschedule failed");
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
