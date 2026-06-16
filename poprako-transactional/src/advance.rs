use async_trait::async_trait;

use crate::{handle::Handle, step::Step};

/// TODO: comment.
#[async_trait]
pub trait Advance<S, H>
where
    S: Step + Send,
    H: Handle + Send,
{
    /// Executes the step with the given handle.
    async fn advance(&self, handle: &mut H, step: S) -> Result<S::Output, S::Error>;
}
