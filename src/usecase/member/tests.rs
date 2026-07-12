// create(create)(positive): team admin should create a member with target user nickname.
// create(create)(negative): non-admin should be rejected without creating a member.
// create(create)(negative): duplicate user and team membership should be rejected.
// list_infos(list_infos)(positive): team member should list team members.
// list_infos(list_infos)(positive): member list should expose user last active timestamp.
// list_infos(list_infos)(positive): role filter should narrow listed members.
// list_infos(list_infos)(positive): pagination should be applied after filtering.
// list_infos(list_infos)(positive): owner should list own memberships.
// list_infos(list_infos)(negative): non-member should be rejected.
// list_infos(list_infos)(negative): invalid list parameter combination should be rejected.
// update_roles(update_roles)(positive): team admin should update member role mask.
// update_roles(update_roles)(negative): non-admin should be rejected without mutation.
// update_roles(update_roles)(negative): missing member should be rejected.
// delete(delete)(positive): team admin should delete a member.
// delete(delete)(negative): non-admin should be rejected without deletion.
// delete(delete)(negative): missing member should be rejected.
// join_team(join_team)(positive): invited user should join team and consume invitation.
// join_team(join_team)(negative): mismatched invitee qid should be rejected without consuming invitation.
// join_team(join_team)(negative): duplicate membership should be rejected without consuming invitation.

use super::*;

use poprako_util::time::ToUnixMilli;

use crate::model::{member_model, team_model, user_model};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{self, assert_expected_variant};
use crate::value::role::{RoleField, RoleMask};

mod join_team;

fn token(user_id: &str) -> user_model::Token {
    user_model::Token {
        user_id: user_id.into(),
    }
}

fn credential(user_id: &str) -> user_model::Credential {
    user_model::Credential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn user(id: &str, nickname: &str) -> user_model::Info {
    //
    let time = test_util::now();

    user_model::Info {
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

fn team(id: &str) -> team_model::Info {
    //
    let time = test_util::now();

    team_model::Info {
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
) -> member_model::Info {
    member_model::Info {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_nickname.into(),
        user_last_active_at: test_util::now(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn create_data(user_id: &str, team_id: &str) -> member_data::CreateData {
    member_data::CreateData {
        user_id: user_id.into(),
        team_id: team_id.into(),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn list_data(team_id: &str) -> member_data::ListInfosData {
    member_data::ListInfosData {
        owner_id: None,
        team_id: Some(team_id.into()),
        fuzzy_nickname: None,
        role: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: 10,
    }
}

fn update_role_data(id: &str) -> member_data::UpdateRolesData {
    member_data::UpdateRolesData {
        id: id.into(),
        roles: RoleMask::from(RoleField::REVIEWER),
    }
}

fn seed_admin(mock: &Mock) {
    mock.seed_member(member(
        "member-admin",
        "admin-user",
        "Admin",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));
}

#[tokio::test]
async fn create_admin_creates_member_with_target_user_nickname() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_team(team("team-1"));

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    let create_outcome = create(
        &mock,
        &mock,
        token("admin-user"),
        create_data("target-user", "team-1"),
    )
    .await;

    assert!(create_outcome.is_ok());

    let created = create_outcome.ok().unwrap();

    let snapshot = mock.snapshot();

    let created_member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == created.id)
        .unwrap();

    assert_eq!(created_member_info.user_id, "target-user");

    assert_eq!(created_member_info.user_nickname, "Target");

    assert_eq!(created_member_info.team_id, "team-1");

    assert_eq!(
        created_member_info.roles,
        RoleMask::from(RoleField::TRANSLATOR)
    );
}

#[tokio::test]
async fn create_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
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
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_team(team("team-1"));

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
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
async fn list_infos_member_lists_team_members() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let mut translator_member_info = member(
        "member-translator",
        "translator-user",
        "Translator",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    let translator_last_active_at = test_util::now();

    translator_member_info.user_last_active_at = translator_last_active_at;

    mock.seed_member(translator_member_info);

    let member_info_vals =
        list_infos(&mock, &mock, token("admin-user"), list_data("team-1"))
            .await;

    assert!(member_info_vals.is_ok());

    let member_info_vals = member_info_vals.ok().unwrap();

    assert_eq!(member_info_vals.len(), 2);

    assert_eq!(member_info_vals[0].id, "member-admin");

    assert_eq!(member_info_vals[1].id, "member-translator");

    assert_eq!(
        member_info_vals[1].last_active_at,
        translator_last_active_at.to_unix_milli()
    );
}

#[tokio::test]
async fn list_infos_filters_by_role() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-reviewer",
        "reviewer-user",
        "Alice Reviewer",
        "team-1",
        RoleMask::from(RoleField::REVIEWER),
    ));

    mock.seed_member(member(
        "member-translator",
        "translator-user",
        "Alice Translator",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let member_info_vals = list_infos(
        &mock,
        &mock,
        token("admin-user"),
        member_data::ListInfosData {
            owner_id: None,
            team_id: Some("team-1".into()),
            fuzzy_nickname: None,
            role: Some(RoleField::REVIEWER),
            incl_opt: Vec::new(),
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
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-reviewer",
        "reviewer-user",
        "Reviewer",
        "team-1",
        RoleMask::from(RoleField::REVIEWER),
    ));

    mock.seed_member(member(
        "member-translator",
        "translator-user",
        "Translator",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let member_info_vals = list_infos(
        &mock,
        &mock,
        token("admin-user"),
        member_data::ListInfosData {
            owner_id: None,
            team_id: Some("team-1".into()),
            fuzzy_nickname: None,
            role: None,
            incl_opt: Vec::new(),
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
async fn list_infos_owner_lists_own_memberships() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-one",
        "user-1",
        "User",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member(
        "member-two",
        "user-1",
        "User",
        "team-2",
        RoleMask::from(RoleField::REVIEWER),
    ));

    mock.seed_member(member(
        "member-other",
        "user-2",
        "Other",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let member_info_vals = list_infos(
        &mock,
        &mock,
        token("user-1"),
        member_data::ListInfosData {
            owner_id: Some("user-1".into()),
            team_id: None,
            fuzzy_nickname: None,
            role: None,
            incl_opt: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .await;

    assert!(member_info_vals.is_ok());

    let member_info_vals = member_info_vals.ok().unwrap();

    assert_eq!(member_info_vals.len(), 2);

    assert_eq!(member_info_vals[0].id, "member-one");

    assert_eq!(member_info_vals[1].id, "member-two");
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err =
        list_infos(&mock, &mock, token("stranger-user"), list_data("team-1"))
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[test]
fn list_infos_rejects_invalid_combination() {
    //
    let err = TryInto::<member_model::ListSpec>::try_into(
        member_data::ListInfosData {
            owner_id: Some("user-1".into()),
            team_id: Some("team-1".into()),
            fuzzy_nickname: None,
            role: None,
            incl_opt: Vec::new(),
            offset: 0,
            limit: 10,
        },
    )
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[test]
fn list_infos_converts_owner_combination_to_mine_spec() {
    //
    let member_list_spec: member_model::ListSpec = member_data::ListInfosData {
        owner_id: Some("user-1".into()),
        team_id: None,
        fuzzy_nickname: None,
        role: None,
        incl_opt: Vec::new(),
        offset: 3,
        limit: 5,
    }
    .try_into()
    .ok()
    .unwrap();

    let member_model::ListSpec::User {
        owner_id,
        offset,
        limit,
        ..
    } = member_list_spec
    else {
        panic!("expected mine list spec");
    };

    assert_eq!(owner_id, "user-1");

    assert_eq!(offset, 3);

    assert_eq!(limit, 5);
}

#[tokio::test]
async fn update_roles_admin_updates_member_role_mask() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let update_member_role = update_roles(
        &mock,
        &mock,
        token("admin-user"),
        update_role_data("member-target"),
    )
    .await;

    assert!(update_member_role.is_ok());

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|m| m.id == "member-target")
        .unwrap();

    assert_eq!(member_info.roles, RoleMask::from(RoleField::REVIEWER));
}

#[tokio::test]
async fn update_roles_non_admin_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err = update_roles(
        &mock,
        &mock,
        token("normal-user"),
        update_role_data("member-target"),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|m| m.id == "member-target")
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(member_info.roles, RoleMask::from(RoleField::TRANSLATOR));
}

#[tokio::test]
async fn update_roles_missing_member_is_rejected() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let err = update_roles(
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
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let delete_member =
        delete(&mock, &mock, token("admin-user"), "member-target".into()).await;

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
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member-normal",
        "normal-user",
        "Normal",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_member(member(
        "member-target",
        "target-user",
        "Target",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err =
        delete(&mock, &mock, token("normal-user"), "member-target".into())
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
    //
    let mock = Mock::new();

    seed_admin(&mock);

    let err =
        delete(&mock, &mock, token("admin-user"), "member-missing".into())
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(mock.snapshot().members.len(), 1);
}
