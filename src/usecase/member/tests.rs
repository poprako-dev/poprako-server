// Member join flows and invitation conversion behavior.
mod join_team;
// Member role updates and deletion scenarios.
mod mutate;

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

use poprako_util::time::ToUnixMilli as _;
use time::Duration;

use crate::data::instr::member::{
    CreateMemberInstr, ListMemberInfosInstr, UpdateMemberRolesInstr,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::read::spec::member::MemberListSpec;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util;
use crate::test_util::assert_expected_variant;
use crate::value::role::{RoleField, RoleMask};

// Build user-token fixture for member scenario authorization.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

// Build deterministic credentials for seeded test users.
fn credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

// Build a user fixture with baseline timestamps and avatar fields.
fn user(id: &str, nickname: &str) -> UserInfo {
    //
    let time = test_util::now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: nickname.into(),
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

// Build a team fixture with baseline metadata.
fn team(id: &str) -> TeamInfo {
    //
    let time = test_util::now();

    TeamInfo {
        id: id.into(),
        name: id.into(),
        description: "description".into(),
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
        created_at: time,
        updated_at: time,
    }
}

// Build a member fixture with a specific role mask and nickname.
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
        user_last_active_at: test_util::now(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

// Build create-member instr for a user/team pair.
fn create_instr(user_id: &str, team_id: &str) -> CreateMemberInstr {
    CreateMemberInstr {
        user_id: user_id.into(),
        team_id: team_id.into(),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

// Build list-member query instr for a specific team.
fn list_instr(team_id: &str) -> ListMemberInfosInstr {
    ListMemberInfosInstr {
        owner_id: None,
        team_id: Some(team_id.into()),
        fuzzy_nickname: None,
        role: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: 10,
    }
}

// Build role-update instr targeting a member.
fn update_role_instr(id: &str) -> UpdateMemberRolesInstr {
    UpdateMemberRolesInstr {
        id: id.into(),
        roles: RoleMask::from(RoleField::REVIEWER),
    }
}

// Seed an admin member used by privileged member operations.
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
        (&mock, &mock),
        token("admin-user"),
        create_instr("target-user", "team-1"),
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
        (&mock, &mock),
        token("normal-user"),
        create_instr("target-user", "team-1"),
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
        (&mock, &mock),
        token("admin-user"),
        create_instr("target-user", "team-1"),
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

    let baseline = test_util::now();

    let mut admin_member_info = member(
        "member-admin",
        "admin-user",
        "Admin",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    );

    admin_member_info.user_last_active_at = baseline - Duration::hours(1);

    mock.seed_member(admin_member_info);

    let mut translator_member_info = member(
        "member-translator",
        "translator-user",
        "Translator",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    let translator_last_active_at = baseline;

    translator_member_info.user_last_active_at = translator_last_active_at;

    mock.seed_member(translator_member_info);

    let member_info_vals =
        list_infos((&mock, &mock), token("admin-user"), list_instr("team-1"))
            .await;

    assert!(member_info_vals.is_ok());

    let member_info_vals = member_info_vals.ok().unwrap();

    assert_eq!(member_info_vals.len(), 2);

    assert_eq!(member_info_vals[0].id, "member-translator");

    assert_eq!(member_info_vals[1].id, "member-admin");

    assert_eq!(
        member_info_vals[0].last_active_at,
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
        (&mock, &mock),
        token("admin-user"),
        ListMemberInfosInstr {
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

    let baseline = test_util::now();

    let mut admin_member_info = member(
        "member-admin",
        "admin-user",
        "Admin",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    );

    admin_member_info.user_last_active_at = baseline;

    mock.seed_member(admin_member_info);

    let mut reviewer_member_info = member(
        "member-reviewer",
        "reviewer-user",
        "Reviewer",
        "team-1",
        RoleMask::from(RoleField::REVIEWER),
    );

    reviewer_member_info.user_last_active_at = baseline - Duration::hours(1);

    mock.seed_member(reviewer_member_info);

    let mut translator_member_info = member(
        "member-translator",
        "translator-user",
        "Translator",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    translator_member_info.user_last_active_at = baseline - Duration::hours(2);

    mock.seed_member(translator_member_info);

    let member_info_vals = list_infos(
        (&mock, &mock),
        token("admin-user"),
        ListMemberInfosInstr {
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

    let baseline = test_util::now();

    let mut first_member_info = member(
        "member-one",
        "user-1",
        "User",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    first_member_info.user_last_active_at = baseline - Duration::hours(1);

    mock.seed_member(first_member_info);

    let mut second_member_info = member(
        "member-two",
        "user-1",
        "User",
        "team-2",
        RoleMask::from(RoleField::REVIEWER),
    );

    second_member_info.user_last_active_at = baseline;

    mock.seed_member(second_member_info);

    mock.seed_member(member(
        "member-other",
        "user-2",
        "Other",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let member_info_vals = list_infos(
        (&mock, &mock),
        token("user-1"),
        ListMemberInfosInstr {
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

    assert_eq!(member_info_vals[0].id, "member-two");

    assert_eq!(member_info_vals[1].id, "member-one");
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

    let err = list_infos(
        (&mock, &mock),
        token("stranger-user"),
        list_instr("team-1"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[test]
fn list_infos_rejects_invalid_combination() {
    //
    let err = TryInto::<MemberListSpec>::try_into(ListMemberInfosInstr {
        owner_id: Some("user-1".into()),
        team_id: Some("team-1".into()),
        fuzzy_nickname: None,
        role: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: 10,
    })
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[test]
fn list_infos_converts_owner_combination_to_mine_spec() {
    //
    let member_list_spec: MemberListSpec = ListMemberInfosInstr {
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

    let MemberListSpec::User {
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
