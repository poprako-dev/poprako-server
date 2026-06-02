use futures_util::FutureExt as _;
use tracing::instrument;

use crate::domain::compound::user::{hash_password, sign_token};
use crate::domain::effect::{Effect as _, EffectSink};
use crate::domain::external::token::TokenSign;
use crate::domain::model::aggregate::member::MemberForm;
use crate::domain::model::aggregate::user::{UserForm, UserToken};
use crate::domain::model::event::{Event, EventSink, user::UserSignedUpEvent};
use crate::domain::query::Transactional;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::query::user::UserQueryTransactional;
use crate::domain::result::DomainError;
use crate::usecase::result::UseCaseResult;
use crate::usecase::value_object::user::{SignUpUserParams, SignUpUserReply};
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[instrument(skip(harn))]
pub async fn sign_up_user<H>(harn: &H, params: SignUpUserParams) -> UseCaseResult<SignUpUserReply>
where
    H: Clone + Transactional + EffectSink + TokenSign + Send + Sync,
{
    // Run the core registration logic inside a database transaction.
    let mut user_form = harn
        .transaction_scoped(move |query| {
            async move {
                // 1. Acquire an exclusive row lock on the pending invitation by its code.
                //    This serialises concurrent attempts to consume the same invitation.
                let invitation = MemberInvitationQueryTransactional::get_by_code_ex(
                    query,
                    &params.invitation_code,
                )
                .await?;

                // 2. Validate the invitee identity.
                if invitation.invitee_qid != params.qid {
                    return Err(DomainError::expected_argument(trl(
                        "error-invalid-invitation-code",
                    )))
                    .trace_debug();
                }

                // 3. Generate password hash.
                let password_hash = hash_password(&params.password)?;

                // 4. Build the UserForm aggregate.
                let mut user_form =
                    UserForm::new(params.qid.clone(), params.nickname.clone(), password_hash);

                // Push domain event (published after commit via effect sink).
                user_form.push_event(Event::UserSignedUp(UserSignedUpEvent {
                    team_id: invitation.team_id.clone(),
                    invitor_id: invitation.invitor_id.clone(),
                    invitee_qid: params.qid.clone(),
                }));

                // 5. Create the user.
                let user = UserQueryTransactional::create(query, &user_form).await?;

                // 6. Create a member record so the user belongs to the team.
                let member_form = MemberForm::new(
                    user.id.clone(),
                    user.nickname.clone(),
                    invitation.team_id.clone(),
                    invitation.roles,
                );

                MemberQueryTransactional::create(query, &member_form).await?;

                // 7. Mark the invitation as consumed.
                MemberInvitationQueryTransactional::mark_pending_as_used(query, &invitation.id)
                    .await?;

                Ok(user_form)
            }
            .boxed()
        })
        .await?;

    // Publish events after successful transaction commit.
    user_form.develop_effect(harn).await;

    // Generate a signed token for the newly registered user.
    let token = sign_token(harn, &UserToken::new(user_form.id.clone()))?;

    Ok(SignUpUserReply {
        user_id: user_form.id,
        token,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use time::OffsetDateTime;

    use super::*;
    use crate::domain::model::aggregate::member_invitation::MemberInvitationAggr;
    use crate::domain::model::event::EventEmit;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::result::{DomainResult, ExpectedVariant};
    use crate::infrastructure::query::memory_mock::MemoryMockQuery;
    use crate::util::DerefTo;

    #[derive(Clone)]
    struct TestHarness {
        query: Arc<MemoryMockQuery>,
        events: Arc<Mutex<Vec<Event>>>,
        token_fails: bool,
    }

    impl TestHarness {
        fn new() -> Self {
            Self {
                query: Arc::new(MemoryMockQuery::new()),
                events: Arc::new(Mutex::new(Vec::new())),
                token_fails: false,
            }
        }

        fn with_token_failure() -> Self {
            Self {
                token_fails: true,
                ..Self::new()
            }
        }

        fn seed_invitation(&self, invitation: MemberInvitationAggr) {
            self.query.seed_member_invitation(invitation);
        }

        fn snapshot(&self) -> crate::infrastructure::query::memory_mock::MemoryMockState {
            self.query.snapshot()
        }

        fn events(&self) -> Vec<Event> {
            self.events.lock().unwrap().clone()
        }
    }

    impl DerefTo for TestHarness {
        type Target = MemoryMockQuery;

        fn deref_to(&self) -> &MemoryMockQuery {
            &self.query
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

    fn invitation(code: &str, invitee_qid: &str, pending: bool) -> MemberInvitationAggr {
        let mask = u32::from(RoleFlag::Admin) | u32::from(RoleFlag::Translator);

        MemberInvitationAggr::new(
            "invitation-1".into(),
            "invitor-1".into(),
            None,
            "team-1".into(),
            invitee_qid.into(),
            code.into(),
            pending,
            RoleMask::from(mask),
            OffsetDateTime::now_utc(),
        )
    }

    fn sign_up_params(code: &str, qid: &str, nickname: &str) -> SignUpUserParams {
        SignUpUserParams {
            qid: qid.into(),
            nickname: nickname.into(),
            password: "secret-password".into(),
            invitation_code: code.into(),
        }
    }

    fn is_expected_argument(err: &crate::usecase::result::UseCaseError) -> bool {
        matches!(
            err.as_ref(),
            DomainError::Expected {
                variant: ExpectedVariant::Argument,
                ..
            }
        )
    }

    fn is_unrecoverable(err: &crate::usecase::result::UseCaseError) -> bool {
        matches!(err.as_ref(), DomainError::Unrecoverable { .. })
    }

    #[tokio::test]
    async fn sign_up_user_creates_user_member_consumes_invitation_emits_event_and_signs_token() {
        let harn = TestHarness::new();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let reply = sign_up_user(
            &harn,
            sign_up_params("CODE123", "invitee-qid", "Invitee"),
        )
        .await
        .unwrap();

        assert_eq!(reply.token, format!("token:{}", reply.user_id));

        let snapshot = harn.snapshot();
        assert_eq!(snapshot.users.len(), 1);
        assert_eq!(snapshot.credentials.len(), 1);
        assert_eq!(snapshot.members.len(), 1);
        assert_eq!(snapshot.member_invitations.len(), 1);

        let user = &snapshot.users[0];
        assert_eq!(user.id, reply.user_id);
        assert_eq!(user.qid, "invitee-qid");
        assert_eq!(user.nickname, "Invitee");

        let credential = &snapshot.credentials[0];
        assert_eq!(credential.qid, "invitee-qid");
        assert!(bcrypt::verify("secret-password", &credential.password_hash).unwrap());

        let member = &snapshot.members[0];
        assert_eq!(member.user_id, reply.user_id);
        assert_eq!(member.team_id, "team-1");
        assert!(member.assigned_admin_at.is_some());
        assert!(member.assigned_translator_at.is_some());
        assert!(member.assigned_proofreader_at.is_none());

        assert!(!snapshot.member_invitations[0].pending);

        let events = harn.events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::UserSignedUp(event) => {
                assert_eq!(event.team_id, "team-1");
                assert_eq!(event.invitor_id, "invitor-1");
                assert_eq!(event.invitee_qid, "invitee-qid");
            }
        }
    }

    #[tokio::test]
    async fn sign_up_user_rejects_missing_invitation_without_side_effects() {
        let harn = TestHarness::new();

        let err = sign_up_user(
            &harn,
            sign_up_params("MISSING", "invitee-qid", "Invitee"),
        )
        .await
        .err()
        .unwrap();

        assert!(is_expected_argument(&err));

        let snapshot = harn.snapshot();
        assert!(snapshot.users.is_empty());
        assert!(snapshot.credentials.is_empty());
        assert!(snapshot.members.is_empty());
        assert!(harn.events().is_empty());
    }

    #[tokio::test]
    async fn sign_up_user_rejects_qid_mismatch_and_rolls_back() {
        let harn = TestHarness::new();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let err = sign_up_user(&harn, sign_up_params("CODE123", "other-qid", "Invitee"))
            .await
            .err()
            .unwrap();

        assert!(is_expected_argument(&err));

        let snapshot = harn.snapshot();
        assert!(snapshot.users.is_empty());
        assert!(snapshot.credentials.is_empty());
        assert!(snapshot.members.is_empty());
        assert!(snapshot.member_invitations[0].pending);
        assert!(harn.events().is_empty());
    }

    #[tokio::test]
    async fn sign_up_user_rolls_back_when_member_create_fails() {
        let harn = TestHarness::new();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let first = sign_up_user(
            &harn,
            sign_up_params("CODE123", "invitee-qid", "Invitee"),
        )
        .await
        .unwrap();
        harn.seed_invitation(invitation("CODE456", "second-qid", true));

        let err = sign_up_user(&harn, sign_up_params("CODE456", "second-qid", "Invitee"))
            .await
            .err()
            .unwrap();

        assert!(matches!(
            err.as_ref(),
            DomainError::Expected {
                variant: ExpectedVariant::Conflict,
                ..
            }
        ));

        let snapshot = harn.snapshot();
        assert_eq!(snapshot.users.len(), 1);
        assert_eq!(snapshot.credentials.len(), 1);
        assert_eq!(snapshot.members.len(), 1);
        assert_eq!(snapshot.users[0].id, first.user_id);
        assert!(snapshot
            .member_invitations
            .iter()
            .find(|inv| inv.code == "CODE456")
            .unwrap()
            .pending);
        assert_eq!(harn.events().len(), 1);
    }

    #[tokio::test]
    async fn sign_up_user_token_failure_happens_after_commit_and_event_publish() {
        let harn = TestHarness::with_token_failure();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let err = sign_up_user(
            &harn,
            sign_up_params("CODE123", "invitee-qid", "Invitee"),
        )
        .await
        .err()
        .unwrap();

        assert!(is_unrecoverable(&err));

        let snapshot = harn.snapshot();
        assert_eq!(snapshot.users.len(), 1);
        assert_eq!(snapshot.credentials.len(), 1);
        assert_eq!(snapshot.members.len(), 1);
        assert!(!snapshot.member_invitations[0].pending);
        assert_eq!(harn.events().len(), 1);
    }
}
