use time::{Duration, OffsetDateTime};

/// Maximum age of a request that is still waiting for external state.
pub const WAIT_TIMEOUT: Duration = Duration::hours(1);

/// Outcome of a local-message task invocation.
pub enum TaskFlow {
    //
    /// Task completed successfully; delete its persisted queue record.
    Complete,

    /// Task encountered a transient error; schedule for retry.
    Retry {
        /// Diagnostic message retained for the next attempt.
        err_message: String,
    },

    /// Task is waiting for external state; reschedule without consuming retry budget.
    Wait {
        /// Diagnostic message retained for the next attempt.
        err_message: String,
    },

    /// Task encountered a fatal error; move record to Dead status.
    Dead {
        /// Diagnostic message retained with the failed task.
        err_message: String,
    },
}

impl TaskFlow {
    /// Stops waiting after the request's deadline, without discarding success.
    pub fn limit_wait(
        self,
        requested_at: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Self {
        //
        match self {
            //
            Self::Wait { err_message }
                if now >= requested_at.saturating_add(WAIT_TIMEOUT) =>
            {
                //
                Self::Dead {
                    err_message: format!(
                        "waiting deadline exceeded: {}",
                        err_message
                    ),
                }
            }

            flow => flow,
        }
    }
}
