use async_trait::async_trait;

use crate::step::Step;

/// TODO: comment.
#[async_trait]
pub trait Advance<S>
where
    S: Step + Send,
{
    /// The handle type of resource.
    type Handle;

    /// Executes the step with the given handle.
    async fn advance(&self, handle: &mut Self::Handle, step: S) -> Result<S::Output, S::Error>;
}

/// TODO: comment.
/// Use &T to prevent clone or Arc wrapper.
#[async_trait]
impl<S, T> Advance<S> for &T
where
    // FIXME: necessary?
    S: Step + Send + 'static,
    T: Advance<S> + Sync + ?Sized,
    T::Handle: Send,
{
    type Handle = <T as Advance<S>>::Handle;

    async fn advance(&self, handle: &mut Self::Handle, step: S) -> Result<S::Output, S::Error> {
        (*self).advance(handle, step).await
    }
}
