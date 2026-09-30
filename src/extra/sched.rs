//! Injected recurring tasks with isolated workers and bounded shutdown.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::FutureExt as _;
use tokio::task::{Id, JoinError, JoinHandle, JoinSet};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::config::sched::{SchedConfig, SchedTaskConfig};
use crate::result::BaseRest;

/// Scheduling decision after a successful attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedNext {
    /// Start another attempt after yielding to the runtime.
    Continue,

    /// Wait for the task's configured idle interval.
    Wait,
}

/// Supplies scheduling policy and independent, mutable runners.
pub trait SchedTask {
    /// Returns policy for this registration; it is not polled at runtime.
    fn config(&self) -> SchedTaskConfig;

    /// Creates one runner per worker, and replaces it after panic or timeout.
    ///
    /// This method must return promptly without blocking the runtime.
    fn runner(&self) -> Box<dyn SchedTaskRunner + Send>;
}

/// Stateful execution owned exclusively by one worker.
#[async_trait]
pub trait SchedTaskRunner {
    /// Executes one attempt; ordinary errors retain this runner for a retry.
    ///
    /// The token requests graceful shutdown. Finish in-flight work and avoid
    /// starting further work once cancelled. Dropping this future must safely
    /// abandon the attempt; do not detach work beyond its lifetime. Record
    /// original error diagnostics at their production leaf before returning.
    async fn run(&mut self, token: CancellationToken) -> BaseRest<SchedNext>;
}

/// Owns cancellation and completion of one background supervisor.
pub struct SchedDesc {
    /// Cancellation signal shared by the supervisor and its workers.
    token: CancellationToken,
    /// Task whose completion includes bounded worker shutdown.
    task: JoinHandle<()>,
}

impl SchedDesc {
    /// Requests cancellation without waiting for completion.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Waits for shutdown and reports a supervisor panic or cancellation.
    ///
    /// After the shared grace period, workers are aborted and get at most one
    /// further second for cooperative cleanup. Non-yielding code cannot be
    /// forcibly stopped by the async runtime.
    ///
    /// # Errors
    /// Returns the supervisor task's join error.
    pub async fn join(mut self) -> Result<(), JoinError> {
        (&mut self.task).await
    }
}

impl Drop for SchedDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Drives heterogeneous tasks without knowing their business dependencies.
pub struct Sched {
    /// Task factories registered before startup.
    tasks: Vec<Box<dyn SchedTask + Send + Sync>>,
    /// Policy for shutting down the complete worker group.
    config: SchedConfig,
}

impl Sched {
    /// Constructs the scheduler without starting workers or creating runners.
    #[must_use]
    pub const fn new(
        tasks: Vec<Box<dyn SchedTask + Send + Sync>>,
        config: SchedConfig,
    ) -> Self {
        Self { tasks, config }
    }

    /// Starts all registered worker groups and returns their runtime owner.
    #[must_use]
    pub fn run_detached(self) -> SchedDesc {
        //
        let token = CancellationToken::new();

        let worker_token = token.clone();

        let task = tokio::spawn(supervise(self, worker_token));

        SchedDesc { token, task }
    }
}

// Retain a factory and its fixed policy for replacing one worker slot.
#[derive(Clone)]
struct Worker {
    // Shared factory; runner state remains private to the worker.
    task: Arc<dyn SchedTask + Send + Sync>,
    // Policy captured once at startup.
    config: SchedTaskConfig,
    // Index local to this task registration.
    index: usize,
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
#[instrument(level = "info", skip_all, fields(task_name = worker.config.name, worker_index = worker.index))]
async fn run_worker(worker: Worker, token: CancellationToken) {
    //
    let mut runner = None;

    loop {
        //
        if token.is_cancelled() {
            return;
        }

        if runner.is_none() {
            //
            let Ok(created) =
                catch_unwind(AssertUnwindSafe(|| worker.task.runner()))
            else {
                //
                tracing::error!(
                    operation = "create_sched_task_runner",
                    "scheduler runner creation panicked; retrying",
                );

                if wait(&token, worker.config.retry_interval).await {
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

        let attempt_rest = timeout(worker.config.timeout, attempt).await;

        let delay = match attempt_rest {
            //
            Ok(Ok(Ok(SchedNext::Continue))) => Duration::ZERO,

            Ok(Ok(Ok(SchedNext::Wait))) => worker.config.idle_interval,

            Ok(Ok(Err(_))) => {
                //
                tracing::warn!(
                    operation = "retry_sched_task",
                    "scheduler attempt failed; retrying",
                );

                worker.config.retry_interval
            }

            Ok(Err(_)) => {
                //
                tracing::error!(
                    operation = "run_sched_task",
                    "scheduler attempt panicked; replacing runner",
                );

                runner = None;

                worker.config.retry_interval
            }

            Err(_) => {
                //
                tracing::warn!(
                    operation = "run_sched_task",
                    "scheduler attempt timed out; replacing runner",
                );

                runner = None;

                worker.config.retry_interval
            }
        };

        if wait(&token, delay).await {
            return;
        }
    }
}

// Register a replacement immediately while its worker waits before execution.
fn spawn_worker(
    workers: &mut JoinSet<()>,
    slots: &mut HashMap<Id, Worker>,
    worker: Worker,
    token: CancellationToken,
    delay: Duration,
) {
    //
    let running_worker = worker.clone();

    let handle = workers.spawn(async move {
        //
        if wait(&token, delay).await {
            return;
        }

        run_worker(running_worker, token).await;
    });

    slots.insert(handle.id(), worker);
}

// Apply one deadline to all workers, then bound cooperative abort cleanup.
async fn shutdown_workers(mut workers: JoinSet<()>, grace: Duration) {
    //
    let drain = async {
        //
        while let Some(worker_join) = workers.join_next().await {
            //
            if worker_join.is_err() {
                //
                tracing::error!(
                    operation = "join_sched_worker",
                    "scheduler worker failed during shutdown",
                );
            }
        }
    };

    if timeout(grace, drain).await.is_err() {
        //
        tracing::warn!("scheduler worker shutdown deadline elapsed");

        workers.abort_all();

        if timeout(Duration::from_secs(1), workers.shutdown())
            .await
            .is_err()
        {
            tracing::error!(
                "scheduler aborted workers did not stop within deadline"
            );
        }
    }
}

// Observe workers in completion order and preserve each registration's capacity.
async fn supervise(sched: Sched, token: CancellationToken) {
    //
    let mut workers = JoinSet::new();

    let mut slots = HashMap::new();

    for task in sched.tasks {
        //
        let config = task.config();

        let task = Arc::<dyn SchedTask + Send + Sync>::from(task);

        for index in 0..config.concurrency.get() {
            //
            let worker = Worker {
                task: task.clone(),
                config,
                index,
            };

            spawn_worker(
                &mut workers,
                &mut slots,
                worker,
                token.clone(),
                Duration::ZERO,
            );
        }
    }

    loop {
        //
        let worker_join = tokio::select! {
            //
            biased;

            () = token.cancelled() => break,

            worker_join = workers.join_next_with_id(), if !workers.is_empty() => worker_join,
        };

        let id = match worker_join {
            //
            Some(Ok((id, ()))) => id,

            Some(Err(err)) => err.id(),

            None => continue,
        };

        let Some(worker) = slots.remove(&id) else {
            continue;
        };

        tracing::warn!(
            task_name = worker.config.name,
            worker_index = worker.index,
            operation = "restart_sched_worker",
            "scheduler worker exited; replacing its slot",
        );

        let delay = worker.config.retry_interval;

        spawn_worker(&mut workers, &mut slots, worker, token.clone(), delay);
    }

    shutdown_workers(workers, sched.config.shutdown_grace).await;
}
