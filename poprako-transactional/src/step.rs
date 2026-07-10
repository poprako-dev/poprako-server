//! Defines the [`Step`] trait, the unit of work within a transactional pipeline.

/// A single unit of work within a transactional chain, parameterized by its
/// [`Output`](Self::Output) type.
pub trait Step {
    /// Output type of the step.
    type Output;
}
