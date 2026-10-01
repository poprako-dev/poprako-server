//! Serial runner attempts with isolated failures and cancellable delays.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt as _;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::config::sched::SchedTaskConfig;
use crate::extra::sched::task::{SchedNext, SchedTask};

/// Waits for the startup delay, then owns one runner until cancellation.
pub async fn run(
    task: Arc<dyn SchedTask + Send + Sync>,
    config: SchedTaskConfig,
    index: usize,
    token: CancellationToken,
    delay: Duration,
) {
    //
    if wait(&token, delay).await {
        return;
    }

    run_worker(task, config, index, token).await;
}

// Keep cancellation responsive even when intervals are configured as zero.
async fn wait(token: &CancellationToken, delay: Duration) -> bool {
    //
    if delay.is_zero() {
        //
        tokio::task::yield_now().await;

        return token.is_cancelled();
    }

    tokio::select! {
        //
        biased;

        () = token.cancelled() => true,

        () = tokio::time::sleep(delay) => false,
    }
}

// Isolate attempts and runner creation without spawning a task per attempt.
#[instrument(level = "info", skip_all, fields(task_name = config.name, worker_index = index))]
async fn run_worker(
    task: Arc<dyn SchedTask + Send + Sync>,
    config: SchedTaskConfig,
    index: usize,
    token: CancellationToken,
) {
    //
    let mut runner = None;

    loop {
        //
        if token.is_cancelled() {
            return;
        }

        if runner.is_none() {
            //
            let Ok(created) = catch_unwind(AssertUnwindSafe(|| task.runner()))
            else {
                //
                tracing::error!(
                    operation = "create_sched_task_runner",
                    "scheduler runner creation panicked; retrying",
                );

                if wait(&token, config.retry_interval).await {
                    return;
                }

                continue;
            };

            runner = Some(created);
        }

        if token.is_cancelled() {
            return;
        }

        let Some(current) = runner.as_mut() else {
            continue;
        };

        let attempt =
            AssertUnwindSafe(async { current.run(token.clone()).await })
                .catch_unwind();

        let attempt_rest = timeout(config.timeout, attempt).await;

        let delay = match attempt_rest {
            //
            Ok(Ok(Ok(SchedNext::Continue))) => Duration::ZERO,

            Ok(Ok(Ok(SchedNext::Wait))) => config.idle_interval,

            Ok(Ok(Err(_))) => {
                //
                tracing::warn!(
                    operation = "retry_sched_task",
                    "scheduler attempt failed; retrying",
                );

                config.retry_interval
            }

            Ok(Err(_)) => {
                //
                tracing::error!(
                    operation = "run_sched_task",
                    "scheduler attempt panicked; replacing runner",
                );

                runner = None;

                config.retry_interval
            }

            Err(_) => {
                //
                tracing::warn!(
                    operation = "run_sched_task",
                    "scheduler attempt timed out; replacing runner",
                );

                runner = None;

                config.retry_interval
            }
        };

        if wait(&token, delay).await {
            return;
        }
    }
}
