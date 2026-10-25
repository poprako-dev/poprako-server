#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// Member join flows and invitation conversion behavior.
mod join_team;
// Member nickname filter conversion behavior.
mod fuzzy_nickname;
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
// update_roles(update_roles)(positive): one of two admins may remove its own admin role.
// update_roles(update_roles)(negative): the last admin role must be retained.
// update_roles(update_roles)(negative): non-admin should be rejected without mutation.
// update_roles(update_roles)(negative): missing member should be rejected.
// delete(delete)(positive): team admin should delete a member.
// delete(delete)(positive): one of two admins may delete its own member.
// delete(delete)(negative): the last admin member must be retained.
// delete(delete)(negative): non-admin should be rejected without deletion.
// delete(delete)(negative): missing member should be rejected.
// join_team(join_team)(positive): invited user should join team and consume invitation.
// join_team(join_team)(negative): mismatched invitee qid should be rejected without consuming invitation.
// join_team(join_team)(negative): duplicate membership should be rejected without consuming invitation.

use super::*;

use time::Duration;

use poprako_util::time::ToUnixMilli as _;

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
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
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
        "member--66825055253",
        "admin-user",
        "Admin",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    ));
}

#[tokio::test]
async fn create_admin_creates_member_with_target_user_nickname() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_team(team("team--00000000001"));

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    let create_outcome = create(
        (&mock, &mock),
        token("admin-user"),
        create_instr("target-user", "team--00000000001"),
    )
    .await;

    assert!(create_outcome.is_ok());

    let created = create_outcome.unwrap();

    let snapshot = mock.snapshot();

    let created_member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.id == created.id)
        .unwrap();

    assert_eq!(created_member_info.user_id, "target-user");

    assert_eq!(created_member_info.user_nickname, "Target");

    assert_eq!(created_member_info.team_id, "team--00000000001");

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
        "member--35634922393",
        "normal-user",
        "Normal",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_team(team("team--00000000001"));

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    let error = create(
        (&mock, &mock),
        token("normal-user"),
        create_instr("target-user", "team--00000000001"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert_eq!(mock.snapshot().members.len(), 1);
}

#[tokio::test]
async fn create_duplicate_member_is_rejected() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_team(team("team--00000000001"));

    mock.seed_user(user("target-user", "Target"), credential("target-user"));

    mock.seed_member(member(
        "member--79630565486",
        "target-user",
        "Target",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let error = create(
        (&mock, &mock),
        token("admin-user"),
        create_instr("target-user", "team--00000000001"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert_eq!(mock.snapshot().members.len(), 2);
}

#[tokio::test]
async fn list_infos_member_lists_team_members() {
    //
    let mock = Mock::new();

    let baseline = test_util::now();

    let mut admin_member_info = member(
        "member--66825055253",
        "admin-user",
        "Admin",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    admin_member_info.user_last_active_at = baseline - Duration::hours(1);

    mock.seed_member(admin_member_info);

    let mut translator_member_info = member(
        "member--35035565321",
        "translator-user",
        "Translator",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    let translator_last_active_at = baseline;

    translator_member_info.user_last_active_at = translator_last_active_at;

    mock.seed_member(translator_member_info);

    let member_info_vals = list_infos(
        (&mock, &mock),
        token("admin-user"),
        list_instr("team--00000000001"),
    )
    .await;

    assert!(member_info_vals.is_ok());

    let member_info_vals = member_info_vals.unwrap();

    assert_eq!(member_info_vals.len(), 2);

    assert_eq!(member_info_vals.first().unwrap().id, "member--35035565321");

    assert_eq!(member_info_vals.get(1).unwrap().id, "member--66825055253");

    assert_eq!(
        member_info_vals.first().unwrap().last_active_at,
        translator_last_active_at.to_unix_milli()
    );
}

#[tokio::test]
async fn list_infos_filters_by_role() {
    //
    let mock = Mock::new();

    seed_admin(&mock);

    mock.seed_member(member(
        "member--19585650251",
        "reviewer-user",
        "Alice Reviewer",
        "team--00000000001",
        RoleMask::from(RoleField::REVIEWER),
    ));

    mock.seed_member(member(
        "member--35035565321",
        "translator-user",
        "Alice Translator",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let member_info_vals = list_infos(
        (&mock, &mock),
        token("admin-user"),
        ListMemberInfosInstr {
            owner_id: None,
            team_id: Some("team--00000000001".into()),
            fuzzy_nickname: None,
            role: Some(RoleField::REVIEWER),
            incl_opt: Vec::new(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await;

    assert!(member_info_vals.is_ok());

    let member_info_vals = member_info_vals.unwrap();

    assert_eq!(member_info_vals.len(), 1);

    assert_eq!(member_info_vals.first().unwrap().id, "member--19585650251");
}

#[tokio::test]
async fn list_infos_applies_pagination_after_filtering() {
    //
    let mock = Mock::new();

    let baseline = test_util::now();

    let mut admin_member_info = member(
        "member--66825055253",
        "admin-user",
        "Admin",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    admin_member_info.user_last_active_at = baseline;

    mock.seed_member(admin_member_info);

    let mut reviewer_member_info = member(
        "member--19585650251",
        "reviewer-user",
        "Reviewer",
        "team--00000000001",
        RoleMask::from(RoleField::REVIEWER),
    );

    reviewer_member_info.user_last_active_at = baseline - Duration::hours(1);

    mock.seed_member(reviewer_member_info);

    let mut translator_member_info = member(
        "member--35035565321",
        "translator-user",
        "Translator",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    translator_member_info.user_last_active_at = baseline - Duration::hours(2);

    mock.seed_member(translator_member_info);

    let member_info_vals = list_infos(
        (&mock, &mock),
        token("admin-user"),
        ListMemberInfosInstr {
            owner_id: None,
            team_id: Some("team--00000000001".into()),
            fuzzy_nickname: None,
            role: None,
            incl_opt: Vec::new(),
            offset: 1,
            limit: crate::value::pagination::PubListLimit::new(1).unwrap(),
        },
    )
    .await;

    assert!(member_info_vals.is_ok());

    let member_info_vals = member_info_vals.unwrap();

    assert_eq!(member_info_vals.len(), 1);

    assert_eq!(member_info_vals.first().unwrap().id, "member--19585650251");
}

#[tokio::test]
async fn list_infos_owner_lists_own_memberships() {
    //
    let mock = Mock::new();

    let baseline = test_util::now();

    let mut first_member_info = member(
        "member--91765445504",
        "user--00000000001",
        "User",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    first_member_info.user_last_active_at = baseline - Duration::hours(1);

    mock.seed_member(first_member_info);

    let mut second_member_info = member(
        "member--13070461154",
        "user--00000000001",
        "User",
        "team--00000000002",
        RoleMask::from(RoleField::REVIEWER),
    );

    second_member_info.user_last_active_at = baseline;

    mock.seed_member(second_member_info);

    mock.seed_member(member(
        "member--85117610840",
        "user--00000000002",
        "Other",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    ));

    let member_info_vals = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListMemberInfosInstr {
            owner_id: Some("user--00000000001".into()),
            team_id: None,
            fuzzy_nickname: None,
            role: None,
            incl_opt: Vec::new(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await;

    assert!(member_info_vals.is_ok());

    let member_info_vals = member_info_vals.unwrap();

    assert_eq!(member_info_vals.len(), 2);

    assert_eq!(member_info_vals.first().unwrap().id, "member--13070461154");

    assert_eq!(member_info_vals.get(1).unwrap().id, "member--91765445504");
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member--79630565486",
        "target-user",
        "Target",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let error = list_infos(
        (&mock, &mock),
        token("stranger-user"),
        list_instr("team--00000000001"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}

#[test]
fn list_infos_rejects_invalid_combination() {
    //
    let error = TryInto::<MemberListSpec>::try_into(ListMemberInfosInstr {
        owner_id: Some("user--00000000001".into()),
        team_id: Some("team--00000000001".into()),
        fuzzy_nickname: None,
        role: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    })
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[test]
fn list_infos_converts_owner_combination_to_mine_spec() {
    //
    let member_list_spec: MemberListSpec = ListMemberInfosInstr {
        owner_id: Some("user--00000000001".into()),
        team_id: None,
        fuzzy_nickname: None,
        role: None,
        incl_opt: Vec::new(),
        offset: 3,
        limit: crate::value::pagination::PubListLimit::new(5).unwrap(),
    }
    .try_into()
    .unwrap();

    assert!(matches!(
        member_list_spec,
        MemberListSpec::User { owner_id, offset, limit, .. }
            if owner_id == "user--00000000001"
                && offset == 3
                && limit.get() == 5
    ));
}
