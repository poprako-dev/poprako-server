#[cfg(test)]
mod tests;

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt as _;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio::time::{Instant, timeout, timeout_at};

/// One worker's delivery with capacity reserved through the entire attempt.
pub struct WorkerSlot<T> {
    /// Topic serviced by this worker.
    work_send: mpsc::UnboundedSender<(T, Instant, OwnedSemaphorePermit)>,
    /// Single execution slot, held before claiming a persisted task.
    capacity: Arc<Semaphore>,
}

impl<T> WorkerSlot<T> {
    /// Creates one worker slot and its receiving delivery.
    pub fn channel() -> (
        Self,
        mpsc::UnboundedReceiver<(T, Instant, OwnedSemaphorePermit)>,
    ) {
        //
        let (work_send, work_recv) = mpsc::unbounded_channel();

        let slot = Self {
            work_send,
            capacity: Arc::new(Semaphore::new(1)),
        };

        (slot, work_recv)
    }

    /// Acquires an idle worker before delivery rows are claimed.
    pub fn acquire(&self) -> Option<OwnedSemaphorePermit> {
        //
        if self.work_send.is_closed() {
            return None;
        }

        self.capacity.clone().try_acquire_owned().ok()
    }

    /// Transfers one claimed task and its reserved capacity to the worker.
    pub fn dispatch(
        &self,
        item: T,
        deadline: Instant,
        permit: OwnedSemaphorePermit,
    ) -> bool {
        self.work_send.send((item, deadline, permit)).is_ok()
    }
}

/// Executes each queued attempt within its claim deadline and isolates panics.
/// Abandoned attempts remain processing until the timed-out attempt is reclaimed.
pub async fn run_worker<T, F, Fut>(
    mut work_recv: mpsc::UnboundedReceiver<(T, Instant, OwnedSemaphorePermit)>,
    completed: Arc<Notify>,
    process: F,
) where
    F: Fn(T) -> Fut,
    Fut: Future<Output = ()>,
{
    //
    while let Some((item, deadline, permit)) = work_recv.recv().await {
        //
        if deadline <= Instant::now() {
            //
            drop(permit);

            completed.notify_one();

            continue;
        }

        let attempt =
            AssertUnwindSafe(async { process(item).await }).catch_unwind();

        match timeout_at(deadline, attempt).await {
            //
            Ok(Ok(())) => {
                //
            }

            Ok(Err(_)) => {
                //
                tracing::error!(
                    "prom attempt panicked; timeout recovery will retry it"
                );
            }

            Err(_) => {
                //
                tracing::warn!(
                    "prom attempt deadline elapsed; timeout recovery will retry it"
                );
            }
        }

        drop(permit);

        completed.notify_one();
    }
}

/// Drains workers within one shared deadline, then aborts unfinished workers.
/// Allows one further second for cooperative cancellation to finish.
pub async fn shutdown_workers(mut workers: JoinSet<()>, grace: Duration) {
    //
    let drain = async {
        //
        while let Some(result) = workers.join_next().await {
            //
            if let Err(error) = result {
                tracing::error!(err = ?error, "prom worker task failed");
            }
        }
    };

    if timeout(grace, drain).await.is_err() {
        //
        tracing::warn!("prom worker shutdown deadline elapsed");

        workers.abort_all();

        if timeout(Duration::from_secs(1), workers.shutdown())
            .await
            .is_err()
        {
            tracing::error!(
                "prom aborted workers did not stop within deadline"
            );
        }
    }
}
