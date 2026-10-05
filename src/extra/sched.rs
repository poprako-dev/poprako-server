//! Injected recurring tasks with isolated workers and bounded shutdown.

// Worker capacity, replacement, and coordinated shutdown.
mod supervisor;
// Serial runner attempts, recovery, and cancellable delays.
mod worker;

/// Scheduling contracts for injected tasks and their runners.
pub mod task;

#[cfg(test)]
mod tests;

use tokio::task::{JoinError, JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::config::sched::SchedConfig;
use crate::extra::sched::task::SchedTask;

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

    /// Requests cancellation and waits for shutdown to complete.
    ///
    /// # Errors
    /// Returns the supervisor task's join error.
    pub async fn cancel_and_join(mut self) -> Result<(), JoinError> {
        //
        self.token.cancel();

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

        let task = tokio::spawn(supervisor::run(
            self.tasks,
            self.config,
            worker_token,
        ));

        SchedDesc { token, task }
    }
}
