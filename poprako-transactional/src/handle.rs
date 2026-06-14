use async_trait::async_trait;

#[async_trait]
pub trait Handle {
    /// The type of error of the handle commit or rollback.
    type Error;

    async fn commit(self) -> Result<(), Self::Error>;

    async fn rollback(self) -> Result<(), Self::Error>;
}
