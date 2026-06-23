use async_trait::async_trait;

#[async_trait]
pub trait DeriveTransactional {
    type Transactional;

    async fn transactional(&self) -> Self::Transactional;
}
