use async_trait::async_trait;

use crate::handle::Handle;

// FIXME: mod rename.

#[async_trait]
pub trait Backend {
    /// Error type of beginning a transaction.
    type Error;

    // TODO: comment.
    type Handle: Handle<Error = Self::Error>;

    async fn begin(&self) -> Result<Self::Handle, Self::Error>;
}
