use async_trait::async_trait;

// NOTE: a connection should be marked broken when handle is constructed,
// and be re-marked !broken when it commits or rollbacks successfully.
// This is because the connection may be used by other transactions, and if the handle is dropped without commit or rollback,
// the connection should be marked broken to prevent other transactions from using it.

#[async_trait]
pub trait Handle {
    /// The type of error of the handle commit or rollback.
    type Error;

    /// The inner type of the handle, which is the implementation of the handle.
    type Data;

    async fn commit(self) -> Result<(), Self::Error>;

    async fn rollback(self) -> Result<(), Self::Error>;
}
