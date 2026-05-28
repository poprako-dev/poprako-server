use futures_util::future::BoxFuture;

use crate::domain::external::image_pool::{ImageDelete, ImageGet, ImagePut};
use crate::domain::query::Transactional;
use crate::domain::result::DomainResl;
use crate::infrastructure::external::image_pool::R2ImagePool;
use crate::infrastructure::query::Query;

pub struct Harn {
    pub query: Query,
    pub image_pool: R2ImagePool,
}

/// Priorited deref to Query for convenient transaction running.
impl std::ops::Deref for Harn {
    type Target = Query;

    fn deref(&self) -> &Self::Target {
        &self.query
    }
}

#[async_trait::async_trait]
impl Transactional for Harn {
    type Query<'a>
        = <Query as Transactional>::Query<'a>
    where
        Harn: 'a;

    #[tracing::instrument(skip(self, f), level = tracing::Level::DEBUG)]
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResl<T>
    where
        T: Send, // Return value must cross .await boundaries; Tokio multi-threaded runtime requires Send
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResl<T>> + Send, // BoxFuture requires the closure to be Send
    {
        <Query as Transactional>::run_in_transaction(&self.query, f).await
    }
}

#[async_trait::async_trait]
impl ImageGet for Harn {
    async fn get_signed(&self, key: &str) -> DomainResl<url::Url> {
        self.image_pool.get_signed(key).await
    }
}

#[async_trait::async_trait]
impl ImagePut for Harn {
    async fn put_signed(&self, key: &str) -> DomainResl<url::Url> {
        self.image_pool.put_signed(key).await
    }
}

#[async_trait::async_trait]
impl ImageDelete for Harn {
    async fn delete_batch(&self, keys: &[&str]) -> DomainResl<()> {
        self.image_pool.delete_batch(keys).await
    }
}
