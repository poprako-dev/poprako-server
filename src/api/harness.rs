use std::sync::Arc;

use async_trait::async_trait;
use futures_util::future::BoxFuture;
use url::Url;

use crate::domain::effect::EffectSink;
use crate::domain::external::image_pool::{ImageDelete, ImageGet, ImagePut};
use crate::domain::external::token::{TokenParse, TokenSign};
use crate::domain::model::aggregate::user::UserToken;
use crate::domain::model::event::EventEmit;
use crate::domain::query::Transactional;
use crate::domain::result::DomainResult;
use crate::infrastructure::effect::{AsyncEffectSink, SharedEffectSink};
use crate::infrastructure::external::image_pool::OssImagePool;
use crate::infrastructure::external::token::JwtCodec;
use crate::infrastructure::query::RdbQuery;
use crate::util::DerefTo;

// ── HarnessInner: shared core for database access, image pool, and token codec ───

pub struct HarnessBase {
    rdb_query: RdbQuery,
    jwt_codec: JwtCodec,
    oss_pool: OssImagePool,
}

impl std::ops::Deref for HarnessBase {
    type Target = RdbQuery;

    fn deref(&self) -> &Self::Target {
        &self.rdb_query
    }
}

#[async_trait]
impl Transactional for HarnessBase {
    type Query<'a>
        = <RdbQuery as Transactional>::Query<'a>
    where
        Self: 'a;

    async fn run_in_transaction<F, T>(&self, f: F) -> DomainResult<T>
    where
        T: Send,
        F: for<'a> FnOnce(&'a mut Self::Query<'a>) -> BoxFuture<'a, DomainResult<T>> + Send,
    {
        self.rdb_query.run_in_transaction(f).await
    }
}

#[async_trait]
impl ImageGet for HarnessBase {
    async fn get_signed(&self, key: &str) -> DomainResult<Url> {
        self.oss_pool.get_signed(key).await
    }
}

#[async_trait]
impl ImagePut for HarnessBase {
    async fn put_signed(&self, key: &str) -> DomainResult<Url> {
        self.oss_pool.put_signed(key).await
    }
}

#[async_trait]
impl ImageDelete for HarnessBase {
    async fn delete_batch(&self, keys: &[&str]) -> DomainResult<()> {
        self.oss_pool.delete_batch(keys).await
    }
}

impl TokenSign for HarnessBase {
    fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
        self.jwt_codec.sign(unsigned_token)
    }
}

impl TokenParse for HarnessBase {
    fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
        self.jwt_codec.parse(signed_token)
    }
}

// ── Harness: public facade with effect sink and Deref to HarnessInner ──────

#[derive(Clone)]
pub struct Harness {
    base: Arc<HarnessBase>,
    effect_sink: SharedEffectSink,
}

impl Harness {
    pub fn new(query: RdbQuery, jwt_codec: JwtCodec, image_pool: OssImagePool) -> Self {
        let base = Arc::new(HarnessBase {
            rdb_query: query,
            jwt_codec,
            oss_pool: image_pool,
        });

        let effect_sink = Arc::new(AsyncEffectSink::new(Arc::clone(&base), 1024));

        Harness { base, effect_sink }
    }
}

impl std::ops::Deref for Harness {
    type Target = HarnessBase;

    fn deref(&self) -> &Self::Target {
        &self.base
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

impl DerefTo for Harness {
    type Target = HarnessBase;

    fn deref_to(&self) -> &HarnessBase {
        &self.base
    }
}
