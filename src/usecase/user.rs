use futures_util::FutureExt as _;
use tracing::instrument;

use crate::domain::compound::user::{hash_password, sign_token};
use crate::domain::effect::{Effect as _, EffectSink};
use crate::domain::external::token::TokenIssuer;
use crate::domain::model::aggregate::member::{MemberAggr, MemberForm};
use crate::domain::model::aggregate::user::{UserAggr, UserForm, UserToken};
use crate::domain::model::event::user::UserSignedUpEvent;
use crate::domain::model::event::{Event, EventSink};
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
    H: Clone + Transactional + EffectSink + TokenIssuer + Send + Sync,
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
                let mut user_form = UserForm::new(
                    UserAggr::generate_id(),
                    params.qid,
                    params.nickname,
                    password_hash,
                );

                let invitation_id = invitation.id;

                user_form.push_event(Event::UserSignedUp(UserSignedUpEvent {
                    team_id: invitation.team_id.clone(),
                    invitor_id: invitation.invitor_id,
                    invitee_qid: user_form.qid.clone(),
                }));

                // 5. Create the user.
                let user = UserQueryTransactional::create(query, &user_form).await?;

                // 6. Create a member record so the user belongs to the team.
                let member_form = MemberForm {
                    id: MemberAggr::generate_id(),
                    user_id: user.id,
                    user_nickname: user.nickname,
                    team_id: invitation.team_id,
                    roles: invitation.roles,
                };

                MemberQueryTransactional::create(query, &member_form).await?;

                // 7. Mark the invitation as consumed.
                MemberInvitationQueryTransactional::mark_pending_as_used(query, &invitation_id)
                    .await?;

                Ok(user_form)
            }
            .boxed()
        })
        .await?;

    // Publish events after successful transaction commit.
    user_form.develop_effect(harn).await;

    // Generate a signed token for the newly registered user.
    let user_token = UserToken {
        user_id: user_form.id,
    };

    let signed_token = sign_token(harn, &user_token)?;

    Ok(SignUpUserReply {
        user_id: user_token.user_id,
        token: signed_token,
    })
}

#[cfg(test)]
mod tests {
    // sign_up_user_creates_user_member_consumes_invitation_emits_event_and_signs_token(sign_up_user)(positive): sign up should succeed when the invitation matches the qid and code.
    // sign_up_user_rejects_missing_invitation_without_side_effects(sign_up_user)(negative): sign up should fail without writes or events when the invitation code is missing.
    // sign_up_user_rejects_qid_mismatch_and_rolls_back(sign_up_user)(negative): sign up should fail and roll back when invitation qid does not match params qid.
    // sign_up_user_rolls_back_when_member_create_fails(sign_up_user)(negative): sign up should roll back user creation when member creation fails.
    // sign_up_user_token_failure_happens_after_commit_and_event_publish(sign_up_user)(negative): token failures should occur after commit and event publication.

    use time::OffsetDateTime;

    use super::sign_up_user;
    use crate::domain::model::aggregate::member_invitation::MemberInvitationAggr;
    use crate::domain::model::event::Event;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::harness::tests::TestHarness;
    use crate::test_util::usecase_is_expected_argument;
    use crate::test_util::usecase_is_expected_conflict;
    use crate::test_util::usecase_is_unrecoverable;
    use crate::usecase::value_object::user::SignUpUserParams;

    fn invitation(code: &str, invitee_qid: &str, pending: bool) -> MemberInvitationAggr {
        let mask = u32::from(RoleFlag::Admin) | u32::from(RoleFlag::Translator);

        MemberInvitationAggr {
            id: "invitation-1".into(),
            invitor_id: "invitor-1".into(),
            invitor: None,
            team_id: "team-1".into(),
            invitee_qid: invitee_qid.into(),
            code: code.into(),
            pending,
            roles: RoleMask::from(mask),
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn sign_up_params(code: &str, qid: &str, nickname: &str) -> SignUpUserParams {
        SignUpUserParams {
            qid: qid.into(),
            nickname: nickname.into(),
            password: "secret-password".into(),
            invitation_code: code.into(),
        }
    }

    #[tokio::test]
    async fn sign_up_user_creates_user_member_consumes_invitation_emits_event_and_signs_token() {
        let harn = TestHarness::default();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let reply = sign_up_user(&harn, sign_up_params("CODE123", "invitee-qid", "Invitee"))
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
        let harn = TestHarness::default();

        let err = sign_up_user(&harn, sign_up_params("MISSING", "invitee-qid", "Invitee"))
            .await
            .err()
            .unwrap();

        assert!(usecase_is_expected_argument(&err));

        let snapshot = harn.snapshot();
        assert!(snapshot.users.is_empty());
        assert!(snapshot.credentials.is_empty());
        assert!(snapshot.members.is_empty());
        assert!(harn.events().is_empty());
    }

    #[tokio::test]
    async fn sign_up_user_rejects_qid_mismatch_and_rolls_back() {
        let harn = TestHarness::default();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let err = sign_up_user(&harn, sign_up_params("CODE123", "other-qid", "Invitee"))
            .await
            .err()
            .unwrap();

        assert!(usecase_is_expected_argument(&err));

        let snapshot = harn.snapshot();
        assert!(snapshot.users.is_empty());
        assert!(snapshot.credentials.is_empty());
        assert!(snapshot.members.is_empty());
        assert!(snapshot.member_invitations[0].pending);
        assert!(harn.events().is_empty());
    }

    #[tokio::test]
    async fn sign_up_user_rolls_back_when_member_create_fails() {
        let harn = TestHarness::default();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let first = sign_up_user(&harn, sign_up_params("CODE123", "invitee-qid", "Invitee"))
            .await
            .unwrap();
        harn.seed_invitation(invitation("CODE456", "second-qid", true));

        let err = sign_up_user(&harn, sign_up_params("CODE456", "second-qid", "Invitee"))
            .await
            .err()
            .unwrap();

        assert!(usecase_is_expected_conflict(&err));

        let snapshot = harn.snapshot();
        assert_eq!(snapshot.users.len(), 1);
        assert_eq!(snapshot.credentials.len(), 1);
        assert_eq!(snapshot.members.len(), 1);
        assert_eq!(snapshot.users[0].id, first.user_id);
        assert!(
            snapshot
                .member_invitations
                .iter()
                .find(|inv| inv.code == "CODE456")
                .unwrap()
                .pending
        );
        assert_eq!(harn.events().len(), 1);
    }

    #[tokio::test]
    async fn sign_up_user_token_failure_happens_after_commit_and_event_publish() {
        let harn = TestHarness::with_token_failure();
        harn.seed_invitation(invitation("CODE123", "invitee-qid", true));

        let err = sign_up_user(&harn, sign_up_params("CODE123", "invitee-qid", "Invitee"))
            .await
            .err()
            .unwrap();

        assert!(usecase_is_unrecoverable(&err));

        let snapshot = harn.snapshot();
        assert_eq!(snapshot.users.len(), 1);
        assert_eq!(snapshot.credentials.len(), 1);
        assert_eq!(snapshot.members.len(), 1);
        assert!(!snapshot.member_invitations[0].pending);
        assert_eq!(harn.events().len(), 1);
    }
}
