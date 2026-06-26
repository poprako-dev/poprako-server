// create(create)(positive): team admin should create a member with target user nickname.
// create(create)(negative): non-admin should be rejected without creating a member.
// create(create)(negative): duplicate user and team membership should be rejected.
// create(create)(negative): invalid role mask should be rejected before mutation.
// list_infos(list_infos)(positive): team member should list team members.
// list_infos(list_infos)(positive): keyword and role filters should narrow listed members.
// list_infos(list_infos)(positive): pagination should be applied after filtering.
// list_infos(list_infos)(negative): non-member should be rejected.
// list_infos(list_infos)(negative): invalid role mask filter should be rejected.
// list_mine_infos(list_mine_infos)(positive): current user memberships should be listed.
// list_mine_infos(list_mine_infos)(positive): missing page should return an empty list.
// update_role(update_role)(positive): team admin should update member role mask.
// update_role(update_role)(negative): non-admin should be rejected without mutation.
// update_role(update_role)(negative): invalid role mask should be rejected before mutation.
// update_role(update_role)(negative): missing member should be rejected.
// delete(delete)(positive): team admin should delete a member.
// delete(delete)(negative): non-admin should be rejected without deletion.
// delete(delete)(negative): missing member should be rejected.

use super::*;

use crate::model::member::MemberInfo;
use crate::model::role::{RoleBit, RoleMask};
use crate::model::team::TeamInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;

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

fn user(id: &str, nickname: &str) -> UserInfo {
    let time = crate::test_util::now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: nickname.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn team(id: &str) -> TeamInfo {
    let time = crate::test_util::now();

    TeamInfo {
        id: id.into(),
        name: id.into(),
        description: "description".into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        workset_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

fn member(
    id: &str,
    user_id: &str,
    user_nickname: &str,
    team_id: &str,
    role_mask: RoleMask,
) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_nickname.into(),
        team_id: team_id.into(),
        role_mask,
    }
}

fn create_data(user_id: &str, team_id: &str) -> CreateMemberData {
    CreateMemberData {
        user_id: user_id.into(),
        team_id: team_id.into(),
        role_mask: RoleBit::TRANSLATOR.0,
    }
}

fn list_data(team_id: &str) -> ListMemberInfosData {
    ListMemberInfosData {
        team_id: team_id.into(),
        user_nickname_keyword: None,
        role_mask: None,
        offset: 0,
        limit: 10,
    }
}

fn list_mine_data(offset: u64, limit: u64) -> ListMineMemberInfosData {
    ListMineMemberInfosData { offset, limit }
}

fn update_role_data(id: &str) -> UpdateMemberRoleData {
    UpdateMemberRoleData {
        id: id.into(),
        role_mask: RoleBit::REVIEWER.0,
    }
}

fn seed_admin(mock: &Mock) {
    mock.seed_member(member(
        "member-admin",
        "admin-user",
        "Admin",
        "team-1",
        RoleMask::from(RoleBit::ADMIN),
    ));
}

#[tokio::test]
async fn create_admin_creates_member_with_target_user_nickname() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_team(team("team-1"));
    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    let create_member_val = create(
        &mock,
        &mock,
        token("admin-user"),
        create_data("target-user", "team-1"),
    )
    .await;
    assert!(create_member_val.is_ok());
    let create_member_val = create_member_val.ok().unwrap();
    let snapshot = mock.snapshot();
    let created_member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == create_member_val.id)
        .unwrap();

    assert_eq!(created_member_info.user_id, "target-user");
    assert_eq!(created_member_info.user_nickname, "Target");
    assert_eq!(created_member_info.team_id, "team-1");
    assert_eq!(
        created_member_info.role_mask,
        RoleMask::from(RoleBit::TRANSLATOR)
    );
}

#[tokio::test]
async fn create_non_admin_is_rejected() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));
    mock.seed_team(team("team-1"));
    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    let err = create(
        &mock,
        &mock,
        token("normal-user"),
        create_data("target-user", "team-1"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
    assert_eq!(mock.snapshot().members.len(), 1);
}

#[tokio::test]
async fn create_duplicate_member_is_rejected() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_team(team("team-1"));
    mock.seed_user(user("target-user", "Target"), credential("target-user"));
    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let err = create(
        &mock,
        &mock,
        token("admin-user"),
        create_data("target-user", "team-1"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(mock.snapshot().members.len(), 2);
}

#[tokio::test]
async fn create_invalid_role_mask_is_rejected() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_team(team("team-1"));
    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    let err = create(
        &mock,
        &mock,
        token("admin-user"),
        CreateMemberData {
            user_id: "target-user".into(),
            team_id: "team-1".into(),
            role_mask: 1 << 31,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(mock.snapshot().members.len(), 1);
}

#[tokio::test]
async fn list_infos_member_lists_team_members() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_member(member(
        "member-translator",
        "translator-user",
        "Translator",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let member_info_vals = list_infos(&mock, token("admin-user"), list_data("team-1")).await;
    assert!(member_info_vals.is_ok());
    let member_info_vals = member_info_vals.ok().unwrap();

    assert_eq!(member_info_vals.len(), 2);
    assert_eq!(member_info_vals[0].id, "member-admin");
    assert_eq!(member_info_vals[1].id, "member-translator");
}

#[tokio::test]
async fn list_infos_filters_by_keyword_and_role() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_member(member(
        "member-reviewer",
        "reviewer-user",
        "Alice Reviewer",
        "team-1",
        RoleMask::from(RoleBit::REVIEWER),
    ));
    mock.seed_member(member(
        "member-translator",
        "translator-user",
        "Alice Translator",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let member_info_vals = list_infos(
        &mock,
        token("admin-user"),
        ListMemberInfosData {
            team_id: "team-1".into(),
            user_nickname_keyword: Some("Alice".into()),
            role_mask: Some(RoleBit::REVIEWER.0),
            offset: 0,
            limit: 10,
        },
    )
    .await;
    assert!(member_info_vals.is_ok());
    let member_info_vals = member_info_vals.ok().unwrap();

    assert_eq!(member_info_vals.len(), 1);
    assert_eq!(member_info_vals[0].id, "member-reviewer");
}

#[tokio::test]
async fn list_infos_applies_pagination_after_filtering() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_member(member(
        "member-reviewer",
        "reviewer-user",
        "Reviewer",
        "team-1",
        RoleMask::from(RoleBit::REVIEWER),
    ));
    mock.seed_member(member(
        "member-translator",
        "translator-user",
        "Translator",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let member_info_vals = list_infos(
        &mock,
        token("admin-user"),
        ListMemberInfosData {
            team_id: "team-1".into(),
            user_nickname_keyword: None,
            role_mask: None,
            offset: 1,
            limit: 1,
        },
    )
    .await;
    assert!(member_info_vals.is_ok());
    let member_info_vals = member_info_vals.ok().unwrap();

    assert_eq!(member_info_vals.len(), 1);
    assert_eq!(member_info_vals[0].id, "member-reviewer");
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let err = list_infos(&mock, token("stranger-user"), list_data("team-1"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn list_infos_invalid_role_mask_is_rejected() {
    let mock = Mock::new();
    seed_admin(&mock);

    let err = list_infos(
        &mock,
        token("admin-user"),
        ListMemberInfosData {
            team_id: "team-1".into(),
            user_nickname_keyword: None,
            role_mask: Some(1 << 31),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_mine_infos_lists_current_user_memberships() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-one",
        "user-1",
        "User",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));
    mock.seed_member(member(
        "member-two",
        "user-1",
        "User",
        "team-2",
        RoleMask::from(RoleBit::REVIEWER),
    ));
    mock.seed_member(member(
        "member-other",
        "user-2",
        "Other",
        "team-1",
        RoleMask::from(RoleBit::ADMIN),
    ));

    let member_info_vals =
        list_mine_infos(&mock, &mock, token("user-1"), list_mine_data(0, 10)).await;
    assert!(member_info_vals.is_ok());
    let member_info_vals = member_info_vals.ok().unwrap();

    assert_eq!(member_info_vals.len(), 2);
    assert_eq!(member_info_vals[0].id, "member-one");
    assert_eq!(member_info_vals[1].id, "member-two");
}

#[tokio::test]
async fn list_mine_infos_returns_empty_for_missing_page() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-one",
        "user-1",
        "User",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let member_info_vals =
        list_mine_infos(&mock, &mock, token("user-1"), list_mine_data(10, 10)).await;
    assert!(member_info_vals.is_ok());
    let member_info_vals = member_info_vals.ok().unwrap();

    assert!(member_info_vals.is_empty());
}

#[tokio::test]
async fn update_role_admin_updates_member_role_mask() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let update_member_role = update_role(
        &mock,
        &mock,
        token("admin-user"),
        update_role_data("member-target"),
    )
    .await;
    assert!(update_member_role.is_ok());
    let snapshot = mock.snapshot();
    let target_member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == "member-target")
        .unwrap();

    assert_eq!(
        target_member_info.role_mask,
        RoleMask::from(RoleBit::REVIEWER)
    );
}

#[tokio::test]
async fn update_role_non_admin_is_rejected() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));
    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let err = update_role(
        &mock,
        &mock,
        token("normal-user"),
        update_role_data("member-target"),
    )
    .await
    .err()
    .unwrap();
    let snapshot = mock.snapshot();
    let target_member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == "member-target")
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
    assert_eq!(
        target_member_info.role_mask,
        RoleMask::from(RoleBit::TRANSLATOR)
    );
}

#[tokio::test]
async fn update_role_invalid_role_mask_is_rejected() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let err = update_role(
        &mock,
        &mock,
        token("admin-user"),
        UpdateMemberRoleData {
            id: "member-target".into(),
            role_mask: 1 << 31,
        },
    )
    .await
    .err()
    .unwrap();
    let snapshot = mock.snapshot();
    let target_member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == "member-target")
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(
        target_member_info.role_mask,
        RoleMask::from(RoleBit::TRANSLATOR)
    );
}

#[tokio::test]
async fn update_role_missing_member_is_rejected() {
    let mock = Mock::new();
    seed_admin(&mock);

    let err = update_role(
        &mock,
        &mock,
        token("admin-user"),
        update_role_data("member-missing"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn delete_admin_deletes_member() {
    let mock = Mock::new();
    seed_admin(&mock);
    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let delete_member = delete(&mock, &mock, token("admin-user"), "member-target".into()).await;
    assert!(delete_member.is_ok());

    assert!(
        !mock
            .snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member-target")
    );
}

#[tokio::test]
async fn delete_non_admin_is_rejected() {
    let mock = Mock::new();
    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));
    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleBit::TRANSLATOR),
    ));

    let err = delete(&mock, &mock, token("normal-user"), "member-target".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
    assert!(
        mock.snapshot()
            .members
            .iter()
            .any(|member_info| member_info.id == "member-target")
    );
}

#[tokio::test]
async fn delete_missing_member_is_rejected() {
    let mock = Mock::new();
    seed_admin(&mock);

    let err = delete(&mock, &mock, token("admin-user"), "member-missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(mock.snapshot().members.len(), 1);
}
