#[cfg(test)]
mod tests;

use time::{Duration, OffsetDateTime};

/// Maximum age of a request that is still waiting for external state.
pub const WAIT_TIMEOUT: Duration = Duration::hours(1);

/// Outcome of a local-message task invocation.
pub enum DispatchFlow {
    /// Task completed successfully; delete its persisted delivery record.
    Complete,

    /// Task encountered a transient error; schedule for retry.
    Retry {
        /// Diagnostic message retained for the next attempt.
        err_msg: String,
    },

    /// Task is waiting for external state; reschedule without consuming retry budget.
    Wait {
        /// Diagnostic message retained for the next attempt.
        err_msg: String,
    },

    /// Task encountered a fatal error; move record to Dead status.
    Dead {
        /// Diagnostic message retained with the failed task.
        err_msg: String,
    },
}

impl DispatchFlow {
    /// Stops waiting after the request's deadline, without discarding success.
    #[must_use]
    pub fn limit_wait(
        self,
        requested_at: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Self {
        //
        match self {
            //
            Self::Wait { err_msg }
                if now >= requested_at.saturating_add(WAIT_TIMEOUT) =>
            {
                Self::Dead {
                    err_msg: format!("waiting deadline exceeded: {}", err_msg),
                }
            }

            flow => flow,
        }
    }
}

/// Enforces the retry limit for a task flow.
///
/// When the task has been retried 3 or more times, transitions from
/// [`DispatchFlow::Retry`] to [`DispatchFlow::Dead`] so the message is not
/// requeued indefinitely.
#[must_use]
pub fn enforce_retry_limit(
    dispatch_flow: DispatchFlow,
    retried_count: i64,
) -> DispatchFlow {
    //
    match (dispatch_flow, retried_count >= 3) {
        //
        (DispatchFlow::Retry { err_msg: error }, true) => {
            DispatchFlow::Dead { err_msg: error }
        }

        (dispatch_flow, _) => dispatch_flow,
    }
}
