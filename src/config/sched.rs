//! Background task scheduling and shutdown policies.

use std::num::NonZeroUsize;
use std::time::Duration;

use serde::Deserialize;

use crate::config::duration;

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
    pub idle_interval_secs: Duration,
    /// Delay after an error, panic, timeout, or unexpected worker exit.
    pub retry_interval_secs: Duration,
}

impl Default for SchedTaskConfig {
    // Use one worker with bounded attempts and short polling intervals.
    fn default() -> Self {
        Self {
            name: "sched_task",
            concurrency: NonZeroUsize::MIN,
            timeout: Duration::from_mins(5),
            idle_interval_secs: Duration::from_secs(5),
            retry_interval_secs: Duration::from_secs(5),
        }
    }
}

/// Shutdown policy shared by every registered task and worker.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SchedConfig {
    /// Shared grace period measured from cancellation, not from each join.
    #[serde(deserialize_with = "duration::deserialize")]
    pub shutdown_grace_secs: Duration,
}

impl Default for SchedConfig {
    // Allow in-flight work to finish before aborting all remaining workers.
    fn default() -> Self {
        Self {
            shutdown_grace_secs: Duration::from_secs(30),
        }
    }
}
