use async_trait::async_trait;
use std::sync::Arc;

use futures_util::future::BoxFuture;

use crate::domain::external::image_pool::{ImageDelete, ImageGet, ImagePut};
use crate::domain::query::Transactional;
use crate::domain::result::DomainResl;
use crate::infrastructure::external::image_pool::OssImagePool;
use crate::infrastructure::external::token::JwtCodec;
use crate::infrastructure::query::Query;
use tracing::Level;
use tracing::instrument;

#[derive(Clone)]
pub struct Harness {
    inner: Arc<HarnessInner>,
}

impl Harness {
    pub fn new(query: Query, jwt_codec: JwtCodec, image_pool: OssImagePool) -> Self {
        Self {
            inner: Arc::new(HarnessInner { query, image_pool }),
        }
    }
}

pub struct HarnessInner {
    pub(crate) query: Query,
    pub(crate) image_pool: OssImagePool,
}

/// Priorited deref to Query for convenient transaction running.
impl std::ops::Deref for Harness {
    type Target = Query;

    fn deref(&self) -> &Self::Target {
        &self.inner.query
    }
}

#[async_trait]
impl Transactional for Harness {
    type Query<'a>
        = <Query as Transactional>::Query<'a>
    where
        Harness: 'a;

    #[instrument(skip(self, f), level = Level::DEBUG)]
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResl<T>
    where
        T: Send, // Return value must cross .await boundaries; Tokio multi-threaded runtime requires Send
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResl<T>> + Send, // BoxFuture requires the closure to be Send
    {
        <Query as Transactional>::run_in_transaction(&self.inner.query, f).await
    }
}

#[async_trait]
impl ImageGet for Harness {
    async fn get_signed(&self, key: &str) -> DomainResl<url::Url> {
        self.inner.image_pool.get_signed(key).await
    }
}

#[async_trait]
impl ImagePut for Harness {
    async fn put_signed(&self, key: &str) -> DomainResl<url::Url> {
        self.inner.image_pool.put_signed(key).await
    }
}

#[async_trait]
impl ImageDelete for Harness {
    async fn delete_batch(&self, keys: &[&str]) -> DomainResl<()> {
        self.inner.image_pool.delete_batch(keys).await
    }
}
