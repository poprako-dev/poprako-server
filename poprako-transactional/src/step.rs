use async_trait::async_trait;

// NOTE: different steps with the same state type can be executed in the same transaction.

/// TODO: comment.
#[async_trait]
pub trait Step {
    /// The state type for the step. It is used by Step to execute itself.
    type State;

    /// The output of the step.
    type Output;

    /// The error type of the step.
    type Error;

    /// The inner type of the step, which is the implementation of the step.
    type Data;
}
