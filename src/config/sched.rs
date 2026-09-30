//! Background task scheduling and shutdown policies.

use std::num::NonZeroUsize;
use std::time::Duration;

/// Immutable scheduling policy, read once when the scheduler starts.
#[derive(Clone, Copy, Debug)]
pub struct SchedTaskConfig {
    /// Stable, non-secret task name used in tracing fields.
    pub name: &'static str,
    /// Number of independent runners active for this task.
    pub concurrency: NonZeroUsize,

    /// Maximum duration of one attempt, including all work inside its runner.
    pub timeout: Duration,
    /// Delay after a successful attempt requests an idle wait.
    pub idle_interval: Duration,
    /// Delay after an error, panic, timeout, or unexpected worker exit.
    pub retry_interval: Duration,
}

impl Default for SchedTaskConfig {
    // Use one worker with bounded attempts and short polling intervals.
    fn default() -> Self {
        Self {
            name: "sched_task",
            concurrency: NonZeroUsize::MIN,
            timeout: Duration::from_mins(5),
            idle_interval: Duration::from_secs(5),
            retry_interval: Duration::from_secs(5),
        }
    }
}

/// Shutdown policy shared by every registered task and worker.
#[derive(Clone, Copy, Debug)]
pub struct SchedConfig {
    /// Shared grace period measured from cancellation, not from each join.
    pub shutdown_grace: Duration,
}

impl Default for SchedConfig {
    // Allow in-flight work to finish before aborting all remaining workers.
    fn default() -> Self {
        Self {
            shutdown_grace: Duration::from_secs(30),
        }
    }
}
