pub mod result;

use async_trait::async_trait;

use crate::manager::result::Error as ScopedError;
use crate::util::AsyncFnMark;

#[async_trait]
pub trait Manager<H> {
    /// The error type.
    type Error;

    async fn transactional_scoped<T, E, F>(&self, f: F) -> Result<T, ScopedError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'h> F: AsyncFnOnce(&'h mut H) -> Result<T, E>
            + AsyncFnMark<&'h mut H, Result<T, E>, Fut: Send>
            + Send;
}
