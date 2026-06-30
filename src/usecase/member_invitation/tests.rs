// create(create)(positive): team admin should create a pending member invitation.
// create(create)(negative): non-admin should be rejected.
// list_infos(list_infos)(positive): team member should list team invitations.
// list_infos(list_infos)(positive): empty contents should return an empty list after membership.
// list_infos(list_infos)(negative): non-member should be rejected.
// update_info(update_info)(positive): team admin should update invitation roles.
// update_info(update_info)(negative): non-admin should be rejected.
// delete(delete)(positive): team admin should delete an invitation.
// delete(delete)(negative): non-admin should be rejected.

use super::*;

use crate::model::member::MemberInfo;
use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::role::{RoleField, RoleMask};
use crate::model::user::{UserCredential, UserInfo};
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::usecase::team::tests::team;

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn user(id: &str, qid: &str) -> UserInfo {
    let time = crate::test_util::now();

    UserInfo {
        id: id.into(),
        qid: qid.into(),
        nickname: id.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn member(id: &str, user_id: &str, team_id: &str, role_mask: RoleMask) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        team_id: team_id.into(),
        roles: role_mask,
    }
}

fn invitation(id: &str, team_id: &str, invitee_qid: &str) -> MemberInvitationInfo {
    MemberInvitationInfo {
        id: id.into(),
        team_id: team_id.into(),
        invitor_id: "admin-user".into(),
        invitee_qid: invitee_qid.into(),
        code: "ABC123".into(),
        pending: true,
        role_mask: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn create_data(team_id: &str, invitee_qid: &str) -> CreateMemberInvitationData {
    CreateMemberInvitationData {
        team_id: team_id.into(),
        invitee_qid: invitee_qid.into(),
        role_mask: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn list_data(team_id: &str) -> ListMemberInvitationInfosData {
    ListMemberInvitationInfosData {
        team_id: team_id.into(),
        pending: Some(true),
        offset: 0,
        limit: 10,
    }
}

fn update_data(id: &str) -> UpdateMemberInvitationInfoData {
    UpdateMemberInvitationInfoData {
        id: id.into(),
        role_mask: RoleMask::from(RoleField::REVIEWER),
    }
}

#[tokio::test]
async fn create_admin_creates_pending_invitation() {
    let mock = Mock::new();
    mock.seed_team(team("team-1", "Team", "Desc"));
    mock.seed_member(member(
        "member-1",
        "admin-user",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));
    mock.seed_user(user("invitee-user", "qid-2"), credential("invitee-user"));

    let result = create(
        &mock,
        &mock,
        token("admin-user"),
        create_data("team-1", "qid-2"),
    )
    .await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.member_invitations.len(), 1);
    assert_eq!(snapshot.member_invitations[0].id, result.id);
    assert_eq!(snapshot.member_invitations[0].invitor_id, "admin-user");
    assert_eq!(snapshot.member_invitations[0].invitee_qid, "qid-2");
    assert!(snapshot.member_invitations[0].pending);
}

#[tokio::test]
async fn create_non_admin_is_rejected() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-1",
        "normal-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err = create(
        &mock,
        &mock,
        token("normal-user"),
        create_data("team-1", "qid-2"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
    assert!(mock.snapshot().member_invitations.is_empty());
}

#[tokio::test]
async fn list_infos_member_lists_invitations() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-1",
        "member-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let result = list_infos(&mock, token("member-user"), list_data("team-1")).await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "inv-1");
}

#[tokio::test]
async fn list_infos_empty_returns_after_membership() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-1",
        "member-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let result = list_infos(&mock, token("member-user"), list_data("team-1")).await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    let mock = Mock::new();
    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let err = list_infos(&mock, token("stranger"), list_data("team-1"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
}

#[tokio::test]
async fn update_info_admin_updates_role_mask() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-1",
        "admin-user",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));
    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let result = update_info(&mock, &mock, token("admin-user"), update_data("inv-1")).await;
    assert!(result.is_ok());

    assert_eq!(
        mock.snapshot().member_invitations[0].role_mask,
        RoleMask::from(RoleField::REVIEWER)
    );
}

#[tokio::test]
async fn update_info_non_admin_is_rejected() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-1",
        "normal-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let err = update_info(&mock, &mock, token("normal-user"), update_data("inv-1"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
}

#[tokio::test]
async fn delete_admin_deletes_invitation() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-1",
        "admin-user",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));
    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let result = delete(&mock, &mock, token("admin-user"), "inv-1".into()).await;
    assert!(result.is_ok());

    assert!(mock.snapshot().member_invitations.is_empty());
}

#[tokio::test]
async fn delete_non_admin_is_rejected() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-1",
        "normal-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let err = delete(&mock, &mock, token("normal-user"), "inv-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::PermDeny);
    assert_eq!(mock.snapshot().member_invitations.len(), 1);
}
