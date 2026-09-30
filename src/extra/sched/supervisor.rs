//! Fixed worker capacity, unexpected-exit recovery, and bounded group shutdown.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::{Id, JoinSet};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::config::sched::{SchedConfig, SchedTaskConfig};
use crate::extra::sched::task::SchedTask;
use crate::extra::sched::worker;

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

/// Preserves each registration's worker count until coordinated shutdown.
pub async fn run(
    tasks: Vec<Box<dyn SchedTask + Send + Sync>>,
    config: SchedConfig,
    token: CancellationToken,
) {
    //
    let mut workers = JoinSet::new();

    let mut slots = HashMap::new();

    for task in tasks {
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

    shutdown_workers(workers, config.shutdown_grace).await;
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

    let handle = workers.spawn(worker::run(
        running_worker.task,
        running_worker.config,
        running_worker.index,
        token,
        delay,
    ));

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
