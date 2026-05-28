use url::Url;

use crate::domain::result::DomainResl;

#[async_trait::async_trait]
pub trait ImageGet {
    async fn get_signed(&self, key: &str) -> DomainResl<Url>;
}

#[async_trait::async_trait]
pub trait ImagePut {
    async fn put_signed(&self, key: &str) -> DomainResl<Url>;
}

#[async_trait::async_trait]
pub trait ImageDelete {
    async fn delete_batch(&self, keys: &[&str]) -> DomainResl<()>;
}
