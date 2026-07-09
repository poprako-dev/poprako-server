use async_trait::async_trait;

use crate::step::Step;

/// Executes a domain step against a transactional context.
///
/// The context is passed as a parameter to [`advance`] rather than being stored
/// in the implementor, which keeps the implementor a plain ZST: no lifetime
/// parameter, no construction ceremony. This makes it a clean DI injection
/// point at the usecase layer.
#[async_trait]
pub trait Advance<S, C>
where
    S: Step,
{
    /// Error type of executing the step.
    type Error;

    /// Executes the step against the given context.
    async fn advance(
        &self,
        context: &mut C,
        step: &S,
    ) -> Result<S::Output, Self::Error>;
}
