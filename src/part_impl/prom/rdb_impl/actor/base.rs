//! Shared types and task dispatch logic for the prom actor submodules.
//!
//! Defined here so that both the parent [`actor`] and its child modules
//! (notably [`pool`]) can import without creating an upward ancestor
//! dependency.
//!
//! [`actor`]: crate::part_impl::prom::rdb_impl::actor
//! [`pool`]: crate::part_impl::prom::rdb_impl::actor::pool

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt as _;
use poprako_orchestra::Nucl;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio::time::{Instant, timeout, timeout_at};
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;

use crate::part::effect::Develop;
use crate::part::nucl::Serial;
use crate::part::obj_dept::PageImage;
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::dispatch;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::result::BaseError;
use crate::shared::RdbContext;

/// One worker's queue with capacity reserved through the entire attempt.
pub struct WorkerSlot<T> {
    //
    /// Queue consumed by this worker.
    work_send: mpsc::UnboundedSender<(T, Instant, OwnedSemaphorePermit)>,
    /// Single execution slot, held before claiming a persisted task.
    capacity: Arc<Semaphore>,
}

impl<T> WorkerSlot<T> {
    /// Creates one worker slot and its receiving queue.
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

    /// Acquires an idle worker before queue rows are claimed.
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
/// Abandoned attempts remain processing until the persisted lease is reclaimed.
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
                    "prom attempt panicked; lease recovery will retry it"
                );
            }

            Err(_) => {
                //
                tracing::warn!(
                    "prom attempt deadline elapsed; lease recovery will retry it"
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

/// Owns cancellation and completion of one background supervisor.
pub struct RdbPromActorDesc {
    //
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
    /// Supervisor task that drains workers with a grace period, then aborts.
    task: tokio::task::JoinHandle<()>,
}

impl RdbPromActorDesc {
    /// Requests cancellation without waiting for completion.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Waits for the supervisor's bounded shutdown and reports join failures.
    /// Workers that exceed the grace period are aborted; non-yielding code
    /// cannot be forcibly stopped by the async runtime.
    ///
    /// # Errors
    /// Returns the supervisor task's join error.
    pub async fn join(mut self) -> Result<(), tokio::task::JoinError> {
        (&mut self.task).await
    }
}

impl Drop for RdbPromActorDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Persisted-task consumer with explicitly injected queue and business ports.
pub struct RdbPromActor<N, R, V, D> {
    //
    /// Queue transaction coordinator.
    prom_nucl: RdbNucl<Serial>,
    /// Queue lifecycle repository.
    prom_repo: RdbPromRepo,

    /// Business transaction coordinator.
    nucl: N,
    /// Business repository.
    repo: R,
    /// Object query port.
    obj_dept_view: V,
    /// Event producer.
    develop: D,

    /// Supervisor cancellation signal.
    token: CancellationToken,
}

impl<N, R, V, D> RdbPromActor<N, R, V, D> {
    /// Constructs a consumer without starting any background work.
    pub fn new(
        (prom_nucl, prom_repo): (RdbNucl<Serial>, RdbPromRepo),
        (nucl, repo, obj_dept_view, develop): (N, R, V, D),
    ) -> Self {
        //
        Self {
            prom_nucl,
            prom_repo,
            nucl,
            repo,
            obj_dept_view,
            develop,
            token: CancellationToken::new(),
        }
    }

    /// Returns the injected queue transaction coordinator.
    pub const fn prom_nucl(&self) -> &RdbNucl<Serial> {
        &self.prom_nucl
    }

    /// Returns the injected queue repository.
    pub const fn prom_repo(&self) -> &RdbPromRepo {
        &self.prom_repo
    }

    /// Returns the supervisor cancellation signal.
    pub const fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl<N, R, V, D> RdbPromActor<N, R, V, D>
where
    N: Nucl<Error = BaseError> + Send + Sync,
    N::Context: Send,
    R: AssignmentInvitationRepo<N::Context>
        + ChapterRepo<N::Context>
        + ChapterWorkflowRecordRepo<N::Context>
        + MemberInvitationRepo<N::Context>
        + PageRepo<N::Context>
        + Send
        + Sync,
    V: ObjDeptView<PageImage, N::Context> + Send + Sync,
    D: Develop + Send + Sync,
{
    /// Decodes and dispatches one persisted prom payload.
    #[instrument(level = "info", skip_all)]
    pub async fn dispatch_payload(
        &self,
        topic: &str,
        payload: &serde_json::Value,
    ) -> TaskFlow {
        //
        let task = match serde_json::from_value::<TaskPayload>(payload.clone())
        {
            //
            Ok(task) => task,

            Err(error) => {
                //
                tracing::error!(
                    operation = "deserialize_prom_payload",
                    sdk_err = ?error,
                    "JSON SDK deserialization error",
                );

                return TaskFlow::Dead {
                    err_message: format!(
                        "failed to deserialize prom payload: {}",
                        error,
                    ),
                };
            }
        };

        let expected_topic = task.topic();

        if topic != expected_topic {
            //
            return TaskFlow::Dead {
                err_message: format!(
                    "prom topic {} does not match payload topic {}",
                    topic, expected_topic
                ),
            };
        }

        dispatch::dispatch::<N::Context, _, _, _, _>(
            (&self.nucl, &self.repo, &self.obj_dept_view, &self.develop),
            task,
        )
        .await
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
        + Sync,
    V: ObjDeptView<PageImage, RdbContext> + Send + Sync,
    D: Develop + Send + Sync,
{
    /// Starts the consumer and transfers shutdown ownership to its descriptor.
    #[must_use]
    pub fn run_detach(self) -> RdbPromActorDesc
    where
        R: 'static,
        V: 'static,
        D: 'static,
    {
        let (token, task) = (self.token.clone(), tokio::spawn(self.run()));

        RdbPromActorDesc { token, task }
    }
}
