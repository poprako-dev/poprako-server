// create(create)(positive): team admin should create a pending member invitation.
// create(create)(negative): non-admin should be rejected.
// list_infos(list_infos)(positive): team member should list team invitations.
// list_infos(list_infos)(positive): empty contents should return an empty list after membership.
// list_infos(list_infos)(negative): non-member should be rejected.
// update_roles(update_roles)(positive): team admin should update invitation roles.
// update_roles(update_roles)(negative): non-admin should be rejected.
// delete(delete)(positive): team admin should delete an invitation.
// delete(delete)(negative): non-admin should be rejected.

use super::*;

use crate::data::instr::member_invitation::{
    CreateMemberInvitationInstr, ListMemberInvitationInfosInstr,
    UpdateMemberInvitationRolesInstr,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::member_invitation::MemberInvitationInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::shared::user::UserToken;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::fixture::team;
use crate::test_util::{self, assert_expected_variant};
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    // Build token fixture for invitation-related operations.
    UserToken {
        user_id: user_id.into(),
    }
}

fn credential(user_id: &str) -> UserCredential {
    // Build a credential fixture with deterministic hash value.
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn user(id: &str, qid: &str) -> UserInfo {
    //
    // Build visible user fixture for creator and invitee flows.
    let time = test_util::now();

    UserInfo {
        id: id.into(),
        qid: qid.into(),
        nickname: id.into(),
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn member(
    id: &str,
    user_id: &str,
    team_id: &str,
    role_mask: RoleMask,
) -> MemberInfo {
    // Build a team membership fixture with given role mask.
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: test_util::now(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn invitation(
    id: &str,
    team_id: &str,
    invitee_qid: &str,
) -> MemberInvitationInfo {
    // Build a pending invitation row for API and lifecycle checks.
    MemberInvitationInfo {
        id: id.into(),
        team_id: team_id.into(),
        invitor: None,
        invitor_id: "admin-user".into(),
        invitee_qid: invitee_qid.into(),
        code: "ABC123".into(),
        is_pending: true,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn create_instr(
    team_id: &str,
    invitee_qid: &str,
) -> CreateMemberInvitationInstr {
    // Build invitation creation instr for success/denial tests.
    CreateMemberInvitationInstr {
        team_id: team_id.into(),
        invitee_qid: invitee_qid.into(),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn list_instr(team_id: &str) -> ListMemberInvitationInfosInstr {
    // Build list instr that request pending invitations with pagination.
    ListMemberInvitationInfosInstr {
        incl_opt: Vec::new(),
        team_id: team_id.into(),
        is_pending: Some(true),
        offset: 0,
        limit: 10,
    }
}

fn update_instr(id: &str) -> UpdateMemberInvitationRolesInstr {
    // Build role-update instr used by invitation mutation tests.
    UpdateMemberInvitationRolesInstr {
        id: id.into(),
        roles: RoleMask::from(RoleField::REVIEWER),
    }
}

#[tokio::test]
async fn create_admin_creates_pending_invitation() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member(
        "member-1",
        "admin-user",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    mock.seed_user(user("invitee-user", "qid-2"), credential("invitee-user"));

    let before = test_util::now();

    let created = create(
        (&mock, &mock, &mock),
        token("admin-user"),
        create_instr("team-1", "qid-2"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.member_invitations.len(), 1);

    assert_eq!(snapshot.member_invitations[0].id, created.id);

    assert_eq!(snapshot.member_invitations[0].invitor_id, "admin-user");

    assert_eq!(snapshot.member_invitations[0].invitee_qid, "qid-2");

    assert!(snapshot.member_invitations[0].is_pending);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(
        snapshot.prom_records[0].payload(),
        TaskPayload::Invitation(InvitationPayload::Member {
            invitation_id: created.id,
        })
    );

    assert!(snapshot.prom_records[0].visible_at() >= before + EXPIRY_DELAY);

    assert!(
        snapshot.prom_records[0].visible_at()
            <= test_util::now() + EXPIRY_DELAY
    );
}

#[tokio::test]
async fn create_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-1",
        "normal-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err = create(
        (&mock, &mock, &mock),
        token("normal-user"),
        create_instr("team-1", "qid-2"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(mock.snapshot().member_invitations.is_empty());

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn list_infos_member_lists_invitations() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-1",
        "member-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let listed =
        list_infos((&mock, &mock), token("member-user"), list_instr("team-1"))
            .await
            .unwrap();

    assert_eq!(listed.len(), 1);

    assert_eq!(listed[0].id, "inv-1");
}

#[tokio::test]
async fn list_infos_empty_returns_after_membership() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-1",
        "member-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let listed =
        list_infos((&mock, &mock), token("member-user"), list_instr("team-1"))
            .await
            .unwrap();

    assert!(listed.is_empty());
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let err =
        list_infos((&mock, &mock), token("stranger"), list_instr("team-1"))
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_roles_admin_updates_role_mask() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-1",
        "admin-user",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    update_roles((&mock, &mock), token("admin-user"), update_instr("inv-1"))
        .await
        .unwrap();

    assert_eq!(
        mock.snapshot().member_invitations[0].roles,
        RoleMask::from(RoleField::REVIEWER)
    );
}

#[tokio::test]
async fn update_roles_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-1",
        "normal-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let err = update_roles(
        (&mock, &mock),
        token("normal-user"),
        update_instr("inv-1"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn delete_admin_deletes_invitation() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-1",
        "admin-user",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    delete((&mock, &mock), token("admin-user"), "inv-1".into())
        .await
        .unwrap();

    assert!(mock.snapshot().member_invitations.is_empty());
}

#[tokio::test]
async fn delete_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-1",
        "normal-user",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member_invitation(invitation("inv-1", "team-1", "qid-2"));

    let err = delete((&mock, &mock), token("normal-user"), "inv-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(mock.snapshot().member_invitations.len(), 1);
}
