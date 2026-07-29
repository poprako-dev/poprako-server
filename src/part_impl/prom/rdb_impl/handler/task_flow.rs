/// Outcome of a topic handler invocation.
pub enum TaskFlow {
    //
    /// Task completed successfully; move record to Completed status.
    Complete,

    /// Task encountered a transient error; schedule for retry.
    Retry(String),

    /// Task encountered a fatal error; move record to Dead status.
    Dead(String),
}
