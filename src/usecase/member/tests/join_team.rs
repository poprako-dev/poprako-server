// join_team(join_team)(positive): invited user should join team and consume invitation.
// join_team(join_team)(negative): mismatched invitee qid should be rejected without consuming invitation.
// join_team(join_team)(negative): duplicate membership should be rejected without consuming invitation.

use super::*;

use crate::data::instr::member::JoinTeamInstr;
use crate::model::read::proj::member_invitation::MemberInvitationInfo;

fn invitation(id: &str, invitee_qid: &str) -> MemberInvitationInfo {
    MemberInvitationInfo {
        id: id.into(),
        team_id: "team-1".into(),
        invitor: None,
        invitor_id: "admin-user".into(),
        invitee_qid: invitee_qid.into(),
        code: "INV123".into(),
        is_pending: true,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn join_team_instr() -> JoinTeamInstr {
    JoinTeamInstr {
        code: "INV123".into(),
    }
}

#[tokio::test]
async fn join_team_invited_user_creates_member_and_consumes_invitation() {
    //
    let mock = Mock::new();

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    mock.seed_member_invitation(invitation("invitation-1", "target-user"));

    super::join_team(
        (&mock, &mock, &mock),
        token("target-user"),
        join_team_instr(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.members.len(), 1);

    assert_eq!(snapshot.members[0].user_id, "target-user");

    assert_eq!(snapshot.members[0].user_nickname, "Target");

    assert_eq!(snapshot.members[0].team_id, "team-1");

    assert_eq!(
        snapshot.members[0].roles,
        RoleMask::from(RoleField::TRANSLATOR)
    );

    assert!(!snapshot.member_invitations[0].is_pending);
}

#[tokio::test]
async fn join_team_mismatched_qid_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    mock.seed_member_invitation(invitation("invitation-1", "other-qid"));

    let err = super::join_team(
        (&mock, &mock, &mock),
        token("target-user"),
        join_team_instr(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.members.is_empty());

    assert!(snapshot.member_invitations[0].is_pending);
}

#[tokio::test]
async fn join_team_duplicate_membership_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member_invitation(invitation("invitation-1", "target-user"));

    let err = super::join_team(
        (&mock, &mock, &mock),
        token("target-user"),
        join_team_instr(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.members.len(), 1);

    assert!(snapshot.member_invitations[0].is_pending);
}
