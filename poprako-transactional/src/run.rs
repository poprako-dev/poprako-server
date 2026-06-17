pub mod result;

use async_trait::async_trait;

use crate::run::result::Error as ScopedError;
use crate::util::AsyncFnMark;

#[async_trait]
pub trait Run<H> {
    /// The error type.
    type Error;

    async fn with_scope<T, E, F>(&self, f: F) -> Result<T, ScopedError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'h> F: AsyncFnOnce(&'h mut H) -> Result<T, E>
            + AsyncFnMark<&'h mut H, Result<T, E>, Fut: Send>
            + Send;
}
