use std::sync::Arc;

use async_trait::async_trait;
use futures_util::future::BoxFuture;
use tracing::Level;
use tracing::instrument;

use crate::domain::effect::EffectSink;
use crate::domain::external::image_pool::{ImageDelete, ImageGet, ImagePut};
use crate::domain::external::token::TokenCodec;
use crate::domain::model::aggregate::user::UserToken;
use crate::domain::model::event::EventEmit;
use crate::domain::query::Transactional;
use crate::domain::result::DomainResult;
use crate::infrastructure::effect::AsyncEffectSink;
use crate::infrastructure::external::image_pool::OssImagePool;
use crate::infrastructure::external::token::JwtCodec;
use crate::infrastructure::query::Query;

#[derive(Clone)]
pub struct Harness {
    inner: Arc<HarnessInner>,
    effect_sink: Arc<AsyncEffectSink>,
}

impl Harness {
    pub fn new(query: Query, jwt_codec: JwtCodec, image_pool: OssImagePool) -> Self {
        let inner = Arc::new(HarnessInner {
            query,
            jwt_codec,
            image_pool,
        });

        let effect_sink = Arc::new(AsyncEffectSink::new(Arc::clone(&inner), 1024));

        Harness { inner, effect_sink }
    }
}

pub struct HarnessInner {
    pub query: Query,
    jwt_codec: JwtCodec,
    image_pool: OssImagePool,
}

impl std::ops::Deref for Harness {
    type Target = HarnessInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// ── Trait impls on HarnessInner (used by bg task for DB access) ────────────

#[async_trait]
impl Transactional for HarnessInner {
    type Query<'a>
        = <Query as Transactional>::Query<'a>
    where
        Self: 'a;

    #[instrument(skip(self, f), level = Level::DEBUG)]
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResult<T>> + Send,
    {
        self.query.run_in_transaction(f).await
    }
}

#[async_trait]
impl ImageGet for HarnessInner {
    async fn get_signed(&self, key: &str) -> DomainResult<url::Url> {
        self.image_pool.get_signed(key).await
    }
}

#[async_trait]
impl ImagePut for HarnessInner {
    async fn put_signed(&self, key: &str) -> DomainResult<url::Url> {
        self.image_pool.put_signed(key).await
    }
}

#[async_trait]
impl ImageDelete for HarnessInner {
    async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()> {
        self.image_pool.delete_batch(keys).await
    }
}

impl TokenCodec for HarnessInner {
    #[instrument(skip(self), level = Level::DEBUG)]
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
        self.jwt_codec.sign(unsigned_token)
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
        self.jwt_codec.parse(signed_token)
    }
}

// ── Delegation impls on Harness (for usecase generic bounds) ───────────────

#[async_trait]
impl Transactional for Harness {
    type Query<'a>
        = <Query as Transactional>::Query<'a>
    where
        Self: 'a;

    #[instrument(skip(self, f), level = Level::DEBUG)]
    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResult<T>> + Send,
    {
        self.inner.run_in_transaction(f).await
    }
}

#[async_trait]
impl EffectSink for Harness {
    async fn handle<E>(&self, src: &mut E)
    where
        E: EventEmit + Send,
    {
        self.effect_sink.handle(src).await
    }
}

impl TokenCodec for Harness {
    #[instrument(skip(self), level = Level::DEBUG)]
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
        self.inner.sign(unsigned_token)
    }

    #[instrument(skip(self), level = Level::DEBUG)]
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
        self.inner.parse(signed_token)
    }
}
