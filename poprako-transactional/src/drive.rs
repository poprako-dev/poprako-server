pub mod result;

use async_trait::async_trait;

use crate::drive::result::Error as DriveError;
use crate::util::AsyncFnMark;

#[async_trait]
pub trait Drive<C> {
    /// The error type.
    type Error;

    async fn with_context<T, E, F>(&self, f: F) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'c> F: AsyncFnOnce(&'c mut C) -> Result<T, E>
            + AsyncFnMark<&'c mut C, Result<T, E>, Fut: Send>
            + Send;
}
