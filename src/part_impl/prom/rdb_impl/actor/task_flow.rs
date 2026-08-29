/// Outcome of a topic actor invocation.
pub enum TaskFlow {
    /// Task completed successfully; move record to Completed status.
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
