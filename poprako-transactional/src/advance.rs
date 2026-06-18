use async_trait::async_trait;

use crate::step::Step;

/// Executes a domain step against a transactional handle.
///
/// The handle is passed as a parameter to [`advance`] rather than being stored
/// in the implementor, which keeps the implementor a plain ZST — no lifetime
/// parameter, no construction ceremony.  This makes it a clean DI injection
/// point at the usecase layer.
#[async_trait]
pub trait Advance<S, H>
where
    S: Step,
{
    /// Error type of executing the step.
    type Error;

    /// Executes the step against the given handle.
    async fn advance(&mut self, handle: &mut H, step: S) -> Result<S::Output, Self::Error>;
}
