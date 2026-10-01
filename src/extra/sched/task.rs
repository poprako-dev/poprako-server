//! Scheduling contracts for injected tasks and their independent runners.

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::config::sched::SchedTaskConfig;
use crate::result::BaseRest;

/// Scheduling decision after a successful attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedNext {
    /// Start another attempt after yielding to the runtime.
    Continue,

    /// Wait for the task's configured idle interval.
    Wait,
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

/// Supplies scheduling policy and independent, mutable runners.
pub trait SchedTask {
    /// Returns policy for this registration; it is not polled at runtime.
    fn config(&self) -> SchedTaskConfig;

    /// Creates one runner per worker, and replaces it after panic or timeout.
    ///
    /// This method must return promptly without blocking the runtime.
    fn runner(&self) -> Box<dyn SchedTaskRunner + Send>;
}
