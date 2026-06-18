pub mod result;

use async_trait::async_trait;

use crate::drive::result::Error as DriveError;
use crate::util::AsyncFnMark;

#[async_trait]
pub trait Drive<H> {
    /// The error type.
    type Error;

    async fn run_transactional<T, E, F>(&self, f: F) -> Result<T, DriveError<E, Self::Error>>
    where
        T: Send,
        E: Send,
        for<'h> F: AsyncFnOnce(&'h mut H) -> Result<T, E>
            + AsyncFnMark<&'h mut H, Result<T, E>, Fut: Send>
            + Send;
}
