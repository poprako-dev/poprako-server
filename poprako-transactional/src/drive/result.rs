//! Error types for the transactional [`Drive`] abstraction, distinguishing
//! step-level failures from backend-level failures.

/// A fallible [`Drive`] result that separates errors originating from within
/// a transactional step from errors originating from the backend that
/// executes the transaction.
pub enum Error<E, BE> {
    /// An error occurred during the execution of a step.
    Advance(E),
    /// An error occurred during the execution of the backend.
    Backend(BE),
}
