use futures_util::FutureExt as _;
use time::Duration;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::domain::complex::user::{hash_password, sign_token};
use crate::domain::effect::{Effect as _, EffectSink};
use crate::domain::external::image_pool::ImageGet;
use crate::domain::external::image_pool::ImagePut;
use crate::domain::external::token::TokenIssuer;
use crate::domain::external::token::TokenSign;
use crate::domain::local_message::message::{ImageLocalMessage, ImageResourceKind};
use crate::domain::model::aggr::member::{MemberAggr, MemberForm};
use crate::domain::model::aggr::user::{UserAggr, UserForm, UserInfoUpdate, UserToken};
use crate::domain::model::event::user::UserSignedUpEvent;
use crate::domain::model::event::{Event, EventSink};
use crate::domain::query::Query;
use crate::domain::query::Transactional;
use crate::domain::query::local_message::LocalMessageQueryTransactional;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::query::user::UserQuery;
use crate::domain::query::user::UserQueryTransactional;
use crate::domain::result::DomainError;
use crate::usecase::data_object::user::{
    MarkAvatarUploadedParams, ReserveAvatarParams, ReserveAvatarReply, SignInParams, SignInReply,
    SignUpParams, SignUpReply, UserBase, UserInfoUpdateParams,
};
use crate::usecase::result::UseCaseResult;

#[instrument(err, skip(harn))]
pub async fn sign_up<H>(harn: &H, params: SignUpParams) -> UseCaseResult<SignUpReply>
where
    H: Clone + Transactional + EffectSink + TokenIssuer + Send + Sync,
{
    // Run the core registration logic inside a database transaction.
    let mut user_form = Transactional::transaction_scoped(harn, move |query| {
        async move {
            // 1. Acquire an exclusive row lock on the pending invitation by its code.
            //    This serialises concurrent attempts to consume the same invitation.
            let invitation =
                MemberInvitationQueryTransactional::get_by_code_ex(query, &params.invitation_code)
                    .await?;

            // 2. Validate the invitee identity.
            if invitation.invitee_qid != params.qid {
                return Err(DomainError::expected_argument(trl(
                    "error-invalid-invitation-code",
                )));
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
            MemberInvitationQueryTransactional::mark_pending_as_used(query, &invitation_id).await?;

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

    Ok(SignUpReply {
        user_id: user_token.user_id,
        token: signed_token,
    })
}

#[instrument(err, skip(harn))]
pub async fn sign_in<H>(harn: &H, params: SignInParams) -> UseCaseResult<SignInReply>
where
    H: Query + TokenSign + Send + Sync,
{
    let credentials = UserQuery::get_credentials_by_qid(harn, &params.qid).await?;

    if !credentials.verify_password(&params.password) {
        return Err(DomainError::expected_authentication(trl("error-wrong-credentials")).into());
    }

    let user_id = credentials.user_id.clone();

    let user_token = UserToken {
        user_id: user_id.clone(),
    };

    let signed_token = sign_token(harn, &user_token)?;

    Ok(SignInReply {
        user_id,
        token: signed_token,
    })
}

#[instrument(err, skip(harn))]
pub async fn get_info<H>(harn: &H, id: &str) -> UseCaseResult<UserBase>
where
    H: Query + ImageGet + Send + Sync,
{
    let user = UserQuery::get_by_id(harn, id).await?;

    let base = UserBase::from_aggr(user, harn).await;

    Ok(base)
}

#[instrument(err, skip(harn))]
pub async fn update_info<H>(
    harn: &H,
    token: UserToken,
    params: UserInfoUpdateParams,
) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    let user_id = token.user_id;

    Transactional::transaction_scoped(harn, move |query| {
        async move {
            let input = UserInfoUpdate {
                id: user_id,
                qid: params.qid,
                nickname: params.nickname,
            };

            UserQueryTransactional::update_info(query, &input).await?;

            MemberQueryTransactional::update_user_nickname(query, &input.id, &input.nickname)
                .await?;
            Ok(())
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn reserve_avatar<H>(
    harn: &H,
    token: UserToken,
    params: ReserveAvatarParams,
) -> UseCaseResult<ReserveAvatarReply>
where
    H: Clone + Transactional + ImagePut + Send + Sync,
{
    let user_id = token.user_id;
    let reservation = Transactional::transaction_scoped(harn, move |query| {
        async move {
            let reservation =
                UserQueryTransactional::reserve_avatar(query, &user_id, &params.file_extension)
                    .await?;

            if let Some(previous_object_key) = reservation.previous_object_key.clone() {
                let message =
                    ImageLocalMessage::delete(previous_object_key).into_form(Duration::seconds(0));
                LocalMessageQueryTransactional::append(query, &message).await?;
            }

            let message = ImageLocalMessage::check_uploaded(
                ImageResourceKind::UserAvatar,
                user_id,
                reservation.object_key.clone(),
                reservation.image_version,
            )
            .into_form(Duration::minutes(15));
            LocalMessageQueryTransactional::append(query, &message).await?;

            Ok(reservation)
        }
        .boxed()
    })
    .await?;

    let put_url = ImagePut::put_signed(harn, &reservation.object_key)
        .await?
        .to_string();

    Ok(ReserveAvatarReply {
        put_url,
        image_version: reservation.image_version,
    })
}

#[instrument(err, skip(harn))]
pub async fn mark_avatar_uploaded<H>(
    harn: &H,
    token: UserToken,
    params: MarkAvatarUploadedParams,
) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    let user_id = token.user_id;
    Transactional::transaction_scoped(harn, move |query| {
        async move {
            UserQueryTransactional::mark_avatar_uploaded(query, &user_id, params.image_version)
                .await?;
            Ok(())
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn touch_last_active<H>(harn: &H, id: &str) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    let owned_id = id.to_owned();

    Transactional::transaction_scoped(harn, move |query| {
        let id = owned_id.clone();

        async move {
            UserQueryTransactional::touch_last_active(query, &id).await?;
            MemberQueryTransactional::touch_last_active(query, &id).await?;

            Ok(())
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // sign_up_user_creates_user_member_consumes_invitation_emits_event_and_signs_token(sign_up_user)(positive): sign up should succeed when the invitation matches the qid and code.
    // sign_up_user_rejects_missing_invitation_without_side_effects(sign_up_user)(negative): sign up should fail without writes or events when the invitation code is missing.
    // sign_up_user_rejects_qid_mismatch_and_rolls_back(sign_up_user)(negative): sign up should fail and roll back when invitation qid does not match params qid.
    // sign_up_user_rolls_back_when_member_create_fails(sign_up_user)(negative): sign up should roll back user creation when member creation fails.
    // sign_up_user_token_failure_happens_after_commit_and_event_publish(sign_up_user)(negative): token failures should occur after commit and event publication.

    use super::sign_up;

    use time::OffsetDateTime;

    use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
    use crate::domain::model::event::Event;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::harness::tests::TestHarness;
    use crate::test_util::usecase_is_expected_argument;
    use crate::test_util::usecase_is_expected_conflict;
    use crate::test_util::usecase_is_unrecoverable;
    use crate::usecase::data_object::user::SignUpParams;

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

    fn sign_up_params(code: &str, qid: &str, nickname: &str) -> SignUpParams {
        SignUpParams {
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

        let reply = sign_up(&harn, sign_up_params("CODE123", "invitee-qid", "Invitee"))
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

        let err = sign_up(&harn, sign_up_params("MISSING", "invitee-qid", "Invitee"))
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

        let err = sign_up(&harn, sign_up_params("CODE123", "other-qid", "Invitee"))
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

        let first = sign_up(&harn, sign_up_params("CODE123", "invitee-qid", "Invitee"))
            .await
            .unwrap();
        harn.seed_invitation(invitation("CODE456", "second-qid", true));

        let err = sign_up(&harn, sign_up_params("CODE456", "second-qid", "Invitee"))
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

        let err = sign_up(&harn, sign_up_params("CODE123", "invitee-qid", "Invitee"))
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

#[cfg(test)]
mod user_use_cases_tests {
    // update_modifies_nickname_and_qid(update_info)(positive): update_info should persist new nickname and qid.
    // update_fails_for_nonexistent_user(update_info)(negative): update_info should fail with expected error for missing user.
    // reserve_avatar_generates_key_and_put_url(reserve_avatar)(positive): reserve_avatar should generate an avatar key and a signed PUT URL.
    // reserve_avatar_fails_for_nonexistent_user(reserve_avatar)(negative): reserve_avatar should fail for missing user.
    // mark_avatar_uploaded_sets_flag(mark_avatar_uploaded)(positive): mark_avatar_uploaded should set the avatar_uploaded flag.
    // mark_avatar_uploaded_fails_for_nonexistent_user(mark_avatar_uploaded)(negative): mark_avatar_uploaded should fail for missing user.
    // touch_last_active_updates_timestamp(touch_last_active)(positive): touch_last_active should update the last active timestamp.
    // touch_last_active_fails_for_nonexistent_user(touch_last_active)(negative): touch_last_active should fail for missing user.

    use super::*;

    use time::OffsetDateTime;

    use crate::domain::local_message::message::{ImageLocalMessage, ImageResourceKind};
    use crate::domain::model::aggr::user::UserCredential;
    use crate::harness::tests::TestHarness;
    use crate::test_util::{usecase_is_expected_argument, usecase_is_unrecoverable};
    use crate::usecase::data_object::user::{
        MarkAvatarUploadedParams, ReserveAvatarParams, SignInParams, UserInfoUpdateParams,
    };

    fn make_test_user(
        id: &str,
        qid: &str,
        nickname: &str,
        password: &str,
    ) -> (UserAggr, UserCredential) {
        let now = OffsetDateTime::now_utc();
        let user = UserAggr {
            id: id.into(),
            nickname: nickname.into(),
            qid: qid.into(),
            is_sadmin: false,
            avatar_key: String::new(),
            avatar_uploaded: false,
            avatar_version: 0,
            last_active_at: now,
            created_at: now,
            updated_at: now,
        };
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
        let credential = UserCredential {
            user_id: id.into(),
            password_hash,
        };
        (user, credential)
    }

    // ── login ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn login_succeeds_with_correct_credentials() {
        let harn = TestHarness::default();
        let (user, credential) = make_test_user("user-1", "qid-1", "Alice", "secret123");
        harn.seed_user(user, credential);

        let reply = sign_in(
            &harn,
            SignInParams {
                qid: "qid-1".into(),
                password: "secret123".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(reply.user_id, "user-1");
        assert_eq!(reply.token, "token:user-1");
    }

    #[tokio::test]
    async fn login_fails_with_wrong_password() {
        let harn = TestHarness::default();
        let (user, credential) = make_test_user("user-1", "qid-1", "Alice", "secret123");
        harn.seed_user(user, credential);

        let err = sign_in(
            &harn,
            SignInParams {
                qid: "qid-1".into(),
                password: "wrong".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(!usecase_is_unrecoverable(&err));
    }

    #[tokio::test]
    async fn login_fails_with_nonexistent_qid() {
        let harn = TestHarness::default();

        let err = sign_in(
            &harn,
            SignInParams {
                qid: "no-such-qid".into(),
                password: "irrelevant".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    // ── get_info ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_info_returns_user_base() {
        let harn = TestHarness::default();
        let (user, credential) = make_test_user("user-1", "qid-1", "Alice", "pw");
        harn.seed_user(user, credential);

        let base = get_info(&harn, "user-1").await.unwrap();

        assert_eq!(base.id, "user-1");
        assert_eq!(base.qid, "qid-1");
        assert_eq!(base.nickname, "Alice");
    }

    #[tokio::test]
    async fn get_info_fails_for_nonexistent_id() {
        let harn = TestHarness::default();

        let err = get_info(&harn, "nonexistent").await.err().unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    // ── update ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_modifies_nickname_and_qid() {
        let harn = TestHarness::default();
        let (user, credential) = make_test_user("user-1", "old-qid", "OldNick", "pw");
        harn.seed_user(user, credential);

        let token = UserToken {
            user_id: "user-1".into(),
        };

        update_info(
            &harn,
            token,
            UserInfoUpdateParams {
                qid: "new-qid".into(),
                nickname: "NewNick".into(),
            },
        )
        .await
        .unwrap();

        let base = get_info(&harn, "user-1").await.unwrap();
        assert_eq!(base.qid, "new-qid");
        assert_eq!(base.nickname, "NewNick");
    }

    #[tokio::test]
    async fn update_fails_for_nonexistent_user() {
        let harn = TestHarness::default();
        let token = UserToken {
            user_id: "nonexistent".into(),
        };

        let err = update_info(
            &harn,
            token,
            UserInfoUpdateParams {
                qid: "q".into(),
                nickname: "n".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    // ── reserve_avatar ────────────────────────────────────────────────────

    #[tokio::test]
    async fn reserve_avatar_generates_key_and_put_url() {
        let harn = TestHarness::default();
        let (user, credential) = make_test_user("user-1", "qid-1", "Alice", "pw");
        harn.seed_user(user, credential);

        let token = UserToken {
            user_id: "user-1".into(),
        };

        let reply = reserve_avatar(
            &harn,
            token,
            ReserveAvatarParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        assert!(reply.put_url.contains("put"));
        assert!(reply.put_url.contains("user_avatar"));
        assert!(reply.put_url.contains("png"));
        assert_eq!(reply.image_version, 1);

        let snapshot = harn.snapshot();
        assert_eq!(snapshot.local_messages.len(), 1);
        let message: ImageLocalMessage =
            serde_json::from_value(snapshot.local_messages[0].payload.clone()).unwrap();
        match message {
            ImageLocalMessage::CheckUploaded {
                resource_kind,
                resource_id,
                object_key,
                image_version,
            } => {
                assert_eq!(resource_kind, ImageResourceKind::UserAvatar);
                assert_eq!(resource_id, "user-1");
                assert_eq!(object_key, "user_avatar/user-1-1.png");
                assert_eq!(image_version, 1);
            }
            ImageLocalMessage::Delete { .. } => panic!("expected check-upload message"),
        }

        // Verify that the avatar_key was persisted.
        let base = get_info(&harn, "user-1").await.unwrap();
        assert!(base.avatar_url.is_none());
    }

    #[tokio::test]
    async fn reserve_avatar_fails_for_nonexistent_user() {
        let harn = TestHarness::default();
        let token = UserToken {
            user_id: "nonexistent".into(),
        };

        let err = reserve_avatar(
            &harn,
            token,
            ReserveAvatarParams {
                file_extension: "png".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    // ── mark_avatar_uploaded ───────────────────────────────────────────────

    #[tokio::test]
    async fn mark_avatar_uploaded_sets_flag() {
        let harn = TestHarness::default();
        let (user, credential) = make_test_user("user-1", "qid-1", "Alice", "pw");
        harn.seed_user(user, credential);
        let token = UserToken {
            user_id: "user-1".into(),
        };

        let reply = reserve_avatar(
            &harn,
            token.clone(),
            ReserveAvatarParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        mark_avatar_uploaded(
            &harn,
            token,
            MarkAvatarUploadedParams {
                image_version: reply.image_version,
            },
        )
        .await
        .unwrap();

        let base = get_info(&harn, "user-1").await.unwrap();
        assert!(base.avatar_url.is_some());
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_fails_for_nonexistent_user() {
        let harn = TestHarness::default();
        let token = UserToken {
            user_id: "nonexistent".into(),
        };

        let err = mark_avatar_uploaded(&harn, token, MarkAvatarUploadedParams { image_version: 1 })
            .await
            .err()
            .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    // ── touch_last_active ──────────────────────────────────────────────────

    #[tokio::test]
    async fn touch_last_active_updates_timestamp() {
        let harn = TestHarness::default();
        let (user, credential) = make_test_user("user-1", "qid-1", "Alice", "pw");
        harn.seed_user(user, credential);

        let before = get_info(&harn, "user-1").await.unwrap().last_active_at;

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

        touch_last_active(&harn, "user-1").await.unwrap();

        let after = get_info(&harn, "user-1").await.unwrap().last_active_at;
        assert!(after > before);
    }

    #[tokio::test]
    async fn touch_last_active_fails_for_nonexistent_user() {
        let harn = TestHarness::default();

        let err = touch_last_active(&harn, "nonexistent").await.err().unwrap();

        assert!(usecase_is_expected_argument(&err));
    }
}
