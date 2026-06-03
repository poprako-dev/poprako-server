use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::effect::EffectSink;
use crate::domain::external::image_pool::{ImageDeleteForward, ImageGetForward, ImagePutForward};
use crate::domain::external::token::{TokenParseForward, TokenSignForward};
use crate::domain::model::event::EventEmit;
use crate::domain::query::TransactionalForward;
use crate::domain::query::system_mail::SystemMailQueryForward;
use crate::domain::query::team::TeamQueryForward;
use crate::domain::query::user::UserQueryForward;
use crate::impl_forward_ref;
use crate::infrastructure::effect::{SharedEffectSink, shared_effect_sink};
use crate::infrastructure::external::image_pool::OssImagePool;
use crate::infrastructure::external::token::JwtIssuer;
use crate::infrastructure::query::RdbQuery;

// ── HarnessBase: shared core for database access, image pool, and token codec ───

pub struct HarnessBase {
    rdb_query: RdbQuery,
    jwt_issuer: JwtIssuer,
    oss_image_pool: OssImagePool,
}

impl_forward_ref!(
    HarnessBase => RdbQuery,
    rdb_query,
    TransactionalForward,
    UserQueryForward,
    TeamQueryForward,
    SystemMailQueryForward,
);

impl_forward_ref!(
    HarnessBase => JwtIssuer,
    jwt_issuer,
    TokenSignForward,
    TokenParseForward,
);

impl_forward_ref!(
    HarnessBase => OssImagePool,
    oss_image_pool,
    ImageGetForward,
    ImagePutForward,
    ImageDeleteForward,
);

// ── Harness: public facade with effect sink and forwarding to HarnessBase ──

#[derive(Clone)]
pub struct Harness {
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

impl_forward_ref!(
    Harness => HarnessBase,
    base,
    TransactionalForward,
    UserQueryForward,
    TeamQueryForward,
    SystemMailQueryForward,
    TokenSignForward,
    TokenParseForward,
    ImageGetForward,
    ImagePutForward,
    ImageDeleteForward,
);

#[cfg(test)]
pub mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::domain::effect::EffectSink;
    use crate::domain::external::token::TokenSign;
    use crate::domain::model::aggregate::member_invitation::MemberInvitationAggr;
    use crate::domain::model::aggregate::user::UserToken;
    use crate::domain::model::event::{Event, EventEmit};
    use crate::domain::query::TransactionalForward;
    use crate::domain::query::user::UserQueryForward;
    use crate::domain::result::{DomainError, DomainResult};
    use crate::impl_forward_ref;
    use crate::infrastructure::query::memory_mock::{MemoryMockQuery, MemoryMockState};

    #[derive(Clone, Default)]
    pub struct TestHarness {
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

        pub fn snapshot(&self) -> MemoryMockState {
            self.query.snapshot()
        }

        pub fn events(&self) -> Vec<Event> {
            self.events.lock().unwrap().clone()
        }
    }

    impl_forward_ref!(
        TestHarness => MemoryMockQuery,
        query,
        TransactionalForward,
        UserQueryForward,
    );

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
}
