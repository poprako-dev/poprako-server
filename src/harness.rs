use std::sync::Arc;

use async_trait::async_trait;
use poprako_macro::ForwardRefs;

use crate::domain::effect::EffectSink;
use crate::domain::external::image_pool::{ImageDeleteForward, ImageGetForward, ImagePutForward};
use crate::domain::external::token::{TokenParseForward, TokenSignForward};
use crate::domain::model::event::EventEmit;
use crate::domain::query::TransactionalForward;
use crate::domain::query::system_mail::SystemMailQueryForward;
use crate::domain::query::team::TeamQueryForward;
use crate::domain::query::user::UserQueryForward;
use crate::infra::effect::{SharedEffectSink, shared_effect_sink};
use crate::infra::external::image_pool::OssImagePool;
use crate::infra::external::token::JwtIssuer;
use crate::infra::query::RdbQuery;

// ── HarnessBase: shared core for database access, image pool, and token codec ───

#[derive(ForwardRefs)]
pub struct HarnessBase {
    #[forward_ref(Transactional, UserQuery, TeamQuery, SystemMailQuery)]
    rdb_query: RdbQuery,

    #[forward_ref(TokenSign, TokenParse)]
    jwt_issuer: JwtIssuer,

    #[forward_ref(ImageGet, ImagePut, ImageDelete)]
    oss_image_pool: OssImagePool,
}

// ── Harness: public facade with effect sink and forwarding to HarnessBase ──

#[derive(Clone, ForwardRefs)]
pub struct Harness {
    #[forward_ref(
        target = HarnessBase,
        Transactional,
        UserQuery,
        TeamQuery,
        SystemMailQuery,
        TokenSign,
        TokenParse,
        ImageGet,
        ImagePut,
        ImageDelete
    )]
    base: Arc<HarnessBase>,

    effect_sink: SharedEffectSink,
}

impl Harness {
    #[allow(clippy::too_many_arguments)]
    pub fn new(query: RdbQuery, token_issuer: JwtIssuer, image_pool: OssImagePool) -> Self {
        let base = Arc::new(HarnessBase {
            rdb_query: query,
            jwt_issuer: token_issuer,
            oss_image_pool: image_pool,
        });

        let effect_sink = shared_effect_sink(Arc::clone(&base), 1024);

        Harness { base, effect_sink }
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

#[cfg(test)]
pub mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use poprako_macro::ForwardRefs;
    use url::Url;

    use crate::domain::effect::EffectSink;
    use crate::domain::external::image_pool::{ImageGet, ImagePut};
    use crate::domain::external::token::TokenParse;
    use crate::domain::external::token::TokenSign;
    use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
    use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserToken};
    use crate::domain::model::event::{Event, EventEmit};
    use crate::domain::query::TransactionalForward;
    use crate::domain::query::system_mail::SystemMailQueryForward;
    use crate::domain::query::team::TeamQueryForward;
    use crate::domain::query::user::UserQueryForward;
    use crate::domain::result::{DomainError, DomainResult};
    use crate::infra::query::memory_mock::{MemoryMockQuery, MemoryMockState};

    #[derive(Clone, Default, ForwardRefs)]
    pub struct TestHarness {
        #[forward_ref(target = MemoryMockQuery, Transactional, UserQuery, TeamQuery, SystemMailQuery)]
        query: Arc<MemoryMockQuery>,
        events: Arc<Mutex<Vec<Event>>>,
        token_fails: bool,
    }

    impl TestHarness {
        pub fn with_token_failure() -> Self {
            Self {
                token_fails: true,
                ..Self::default()
            }
        }

        pub fn seed_invitation(&self, invitation: MemberInvitationAggr) {
            self.query.seed_member_invitation(invitation);
        }

        pub fn seed_user(&self, user: UserAggr, credential: UserCredential) {
            self.query.seed_user(user, credential);
        }

        pub fn snapshot(&self) -> MemoryMockState {
            self.query.snapshot()
        }

        pub fn events(&self) -> Vec<Event> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl EffectSink for TestHarness {
        async fn handle<E>(&self, src: &mut E)
        where
            E: EventEmit + Send,
        {
            self.events.lock().unwrap().extend(src.pull_events());
        }
    }

    impl TokenSign for TestHarness {
        fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
            if self.token_fails {
                return Err(DomainError::unrecoverable("token failed".into()));
            }

            Ok(format!("token:{}", unsigned_token.user_id))
        }
    }

    impl TokenParse for TestHarness {
        fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
            if self.token_fails {
                return Err(DomainError::unrecoverable("token failed".into()));
            }

            Ok(UserToken {
                user_id: signed_token.replace("token:", ""),
            })
        }
    }

    #[async_trait]
    impl ImageGet for TestHarness {
        async fn get_signed(&self, key: &str) -> DomainResult<Url> {
            Ok(Url::parse(&format!("https://test.test/get/{}", key)).unwrap())
        }
    }

    #[async_trait]
    impl ImagePut for TestHarness {
        async fn put_signed(&self, key: &str) -> DomainResult<Url> {
            Ok(Url::parse(&format!("https://test.test/put/{}", key)).unwrap())
        }
    }
}
