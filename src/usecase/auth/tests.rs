//! Test fixtures and cases for the authentication use case module.
//!
//! Tests exercise the [`register`] and [`login`] functions against a
//! [`Mock`] that doubles as the driver, repository, token authority,
//! and effect developer.
//!
//! [`register`]: super::register
//! [`login`]: super::login
//! [`Mock`]: crate::part_impl::repo::mock_impl::Mock

// register(register)(positive): pending invitation should create a user and member, consume the invitation, emit signup, and return a token.
// register(register)(negative): qid mismatch should rollback user and member creation without consuming the invitation.
// register(register)(negative): token signing failure should propagate after the transaction and signup event finish.
// login(login)(positive): matching credentials should return the user id and signed token.
// login(login)(negative): missing user should propagate an argument error.
// login(login)(negative): wrong password should return an auth error.
// login(login)(negative): token signing failure should return an auth error.

use super::*;

use crate::model::member_invitation::MemberInvitationInfo;
use crate::part::effect::event::Event;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::usecase::team::tests::team;
use crate::usecase::user::tests::{credential, invalid_credential, user};
use crate::value::role::{RoleField, RoleMask};

/// Builds a pending [`MemberInvitationInfo`] fixture.
fn invitation(
    id: &str,
    team_id: &str,
    invitor_id: &str,
    invitee_qid: &str,
    code: &str,
    pending: bool,
) -> MemberInvitationInfo {
    MemberInvitationInfo {
        id: id.into(),
        team_id: team_id.into(),
        invitor: None,
        invitor_id: invitor_id.into(),
        invitee_qid: invitee_qid.into(),
        code: code.into(),
        pending,
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
    }
}

/// Builds a [`RegisterData`] fixture with a fixed password.
fn register_data(qid: &str, nickname: &str, code: &str) -> RegisterData {
    RegisterData {
        qid: qid.into(),
        nickname: nickname.into(),
        password: "password".into(),
        code: code.into(),
    }
}

/// Builds a [`LoginData`] fixture.
fn login_data(qid: &str, password: &str) -> LoginData {
    LoginData {
        qid: qid.into(),
        password: password.into(),
    }
}

#[tokio::test]
async fn register_creates_user_member_consumes_invitation_and_emits_signup() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member_invitation(invitation(
        "invitation-1",
        "team-1",
        "invitor-1",
        "qid-1",
        "code-1",
        true,
    ));

    let val = register(
        &mock,
        &mock,
        &mock,
        &mock,
        register_data("qid-1", "Nick", "code-1"),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(val.token, format!("token:{}", val.user_id));

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);

    assert_eq!(snapshot.users[0].id, val.user_id);

    assert_eq!(snapshot.members.len(), 1);

    assert_eq!(snapshot.members[0].user_id, val.user_id);

    assert!(!snapshot.member_invitations[0].pending);

    let events = mock.drain_events();

    assert_eq!(events.len(), 1);

    let Event::UserSignedUp(payload) = &events[0] else {
        panic!("expected UserSignedUp event");
    };

    assert_eq!(payload.team_id, "team-1");

    assert_eq!(payload.invitor_id, "invitor-1");

    assert_eq!(payload.invitee_qid, "qid-1");
}

#[tokio::test]
async fn register_rolls_back_when_invitee_qid_mismatches() {
    //
    let mock = Mock::new();

    mock.seed_member_invitation(invitation(
        "invitation-1",
        "team-1",
        "invitor-1",
        "qid-1",
        "code-1",
        true,
    ));

    let err = register(
        &mock,
        &mock,
        &mock,
        &mock,
        register_data("other-qid", "Nick", "code-1"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());

    assert!(snapshot.members.is_empty());

    assert!(snapshot.member_invitations[0].pending);

    assert_eq!(mock.event_count(), 0);
}

#[tokio::test]
async fn register_propagates_token_failure_after_commit_and_event() {
    //
    let mock = Mock::new().with_token_failure();

    mock.seed_member_invitation(invitation(
        "invitation-1",
        "team-1",
        "invitor-1",
        "qid-1",
        "code-1",
        true,
    ));

    let err = register(
        &mock,
        &mock,
        &mock,
        &mock,
        register_data("qid-1", "Nick", "code-1"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Auth);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);

    assert_eq!(snapshot.members.len(), 1);

    assert!(!snapshot.member_invitations[0].pending);

    assert_eq!(mock.event_count(), 1);
}

#[tokio::test]
async fn login_returns_signed_token_for_matching_credentials() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let val = login(&mock, &mock, login_data("qid-1", "password"))
        .await
        .ok()
        .unwrap();

    assert_eq!(val.user_id, "user-1");

    assert_eq!(val.token, "token:user-1");
}

#[tokio::test]
async fn login_propagates_missing_user() {
    //
    let mock = Mock::new();

    let err = login(&mock, &mock, login_data("qid-1", "password"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn login_rejects_wrong_password() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        invalid_credential("user-1"),
    );

    let err = login(&mock, &mock, login_data("qid-1", "password"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Auth);
}

#[tokio::test]
async fn login_propagates_token_failure() {
    //
    let mock = Mock::new().with_token_failure();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let err = login(&mock, &mock, login_data("qid-1", "password"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Auth);
}
