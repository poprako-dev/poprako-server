//! Runtime policy for the hierarchy cleanup task.

use std::num::NonZeroUsize;
use std::time::Duration;

use anyhow::ensure;
use serde::Deserialize;

use crate::config::duration;
use crate::config::sched::SchedTaskConfig;

/// Scheduling settings for subtree deletion; the task identity is fixed.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SubtreeDeleteConfig {
    /// Number of independent cleanup runners.
    pub concurrency: NonZeroUsize,

    /// Maximum duration of one cleanup attempt.
    #[serde(
        rename = "timeout_secs",
        deserialize_with = "duration::deserialize"
    )]
    pub timeout: Duration,
    /// Delay when no cleanup work remains.
    #[serde(
        rename = "idle_interval_secs",
        deserialize_with = "duration::deserialize"
    )]
    pub idle_interval: Duration,
    /// Delay after a failed attempt or worker exit.
    #[serde(
        rename = "retry_interval_secs",
        deserialize_with = "duration::deserialize"
    )]
    pub retry_interval: Duration,
}

impl SubtreeDeleteConfig {
    /// Rejects intervals that would time out immediately or poll continuously.
    ///
    /// # Errors
    /// Returns an error when the timeout, idle interval or retry interval is zero.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        for (field, duration) in [
            ("timeout_secs", self.timeout),
            ("idle_interval_secs", self.idle_interval),
            ("retry_interval_secs", self.retry_interval),
        ] {
            //
            ensure!(
                !duration.is_zero(),
                "subtree_delete.{field} must be positive"
            );
        }

        Ok(())
    }

    /// Converts the configured policy into the scheduler's task descriptor.
    #[must_use]
    pub const fn task_config(self) -> SchedTaskConfig {
        SchedTaskConfig {
            name: "subtree_delete",
            concurrency: self.concurrency,
            timeout: self.timeout,
            idle_interval: self.idle_interval,
            retry_interval: self.retry_interval,
        }
    }
}

impl Default for SubtreeDeleteConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        //
        let task = SchedTaskConfig::default();

        Self {
            concurrency: NonZeroUsize::MIN.saturating_add(1),
            timeout: task.timeout,
            idle_interval: task.idle_interval,
            retry_interval: task.retry_interval,
        }
    }
}
