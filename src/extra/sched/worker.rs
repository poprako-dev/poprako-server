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

// Why one isolated attempt produced no scheduling decision.
#[derive(Clone, Copy)]
enum AttemptFault {
    /// The runner reported an ordinary error and stays reusable.
    Failed,

    /// The attempt panicked, so its runner is discarded.
    Panicked,

    /// The attempt exceeded its timeout, so its runner is discarded.
    TimedOut,
}

impl AttemptFault {
    // Whether recovering from this fault requires a fresh runner.
    const fn replaces_runner(self) -> bool {
        matches!(self, Self::Panicked | Self::TimedOut)
    }

    // Records the fault at the level matching its severity.
    fn report(self) {
        //
        match self {
            //
            Self::Failed => tracing::warn!(
                operation = "retry_sched_task",
                "scheduler attempt failed; retrying",
            ),

            Self::Panicked => tracing::error!(
                operation = "run_sched_task",
                "scheduler attempt panicked; replacing runner",
            ),

            Self::TimedOut => tracing::warn!(
                operation = "run_sched_task",
                "scheduler attempt timed out; replacing runner",
            ),
        }
    }
}

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
#[instrument(level = "info", skip_all, fields(task_name = config.name,
    worker_index = index))]
async fn run_worker(
    task: Arc<dyn SchedTask + Send + Sync>,
    config: SchedTaskConfig,
    index: usize,
    token: CancellationToken,
) {
    //
    loop {
        //
        if token.is_cancelled() {
            return;
        }

        // The runner's scope lasts until an isolation fault discards it.
        {
            let Ok(mut current) =
                catch_unwind(AssertUnwindSafe(|| task.runner()))
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

            if token.is_cancelled() {
                return;
            }

            loop {
                //
                let attempt = AssertUnwindSafe(async {
                    current.run(token.clone()).await
                })
                .catch_unwind();

                // Peel timeout and panic isolation so only the attempt
                // decision remains.
                let settled = match timeout(config.timeout, attempt).await {
                    //
                    Ok(Ok(decision)) => {
                        decision.map_err(|_| AttemptFault::Failed)
                    }

                    Ok(Err(_)) => Err(AttemptFault::Panicked),

                    Err(_) => Err(AttemptFault::TimedOut),
                };

                let delay = match settled {
                    //
                    Ok(SchedNext::Continue) => Duration::ZERO,

                    Ok(SchedNext::Wait) => config.idle_interval,

                    Err(fault) => {
                        //
                        fault.report();

                        if fault.replaces_runner() {
                            break;
                        }

                        config.retry_interval
                    }
                };

                if wait(&token, delay).await {
                    return;
                }
            }
        }

        // The discarded runner is gone before its replacement delay starts.
        if wait(&token, config.retry_interval).await {
            return;
        }
    }
}
