/// TODO: comment.
pub trait Step {
    /// The output of the step.
    type Output;

    /// The error type of the step.
    type Error;
}
