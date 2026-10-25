#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

//! Test fixtures and cases for the team use case module.
//!
//! Tests exercise team CRUD, avatar management, and deletion against
//! a [`Mock`] that doubles as the coordinator, repository, prom enqueuer,
//! and image pool. Failure flags simulate repository errors.
//!
//! [`Mock`]: crate::part_impl::repo::mock_impl::Mock

// Team avatar reservation, upload check, and cleanup behavior.
mod avatar;

// create(create)(positive): creating a team should persist it and return team info.
// create(create)(positive): creating a team should make creator an admin member.
// create(create)(negative): create repo failure should propagate.
// get_info(get_info)(positive): existing team should return info with avatar URL when uploaded.
// get_info(get_info)(negative): missing team should propagate an argument error.
// list_infos(list_infos)(positive): list should return paged teams in repo order.
// list_infos(list_infos)(negative): missing page contents should return an empty list.
// list_infos(list_infos)(negative): listing all teams should require super-admin perm.
// list_infos(list_infos)(negative): filtering by another user should be rejected.
// update_info(update_info)(positive): existing team should update name and description.
// update_info(update_info)(negative): missing team should propagate an argument error.
// reserve_avatar(reserve_avatar)(positive): first reservation should update avatar state, enqueue a check, and return a put URL.
// reserve_avatar(reserve_avatar)(positive): replacing an avatar should enqueue delete and check messages.
// reserve_avatar(reserve_avatar)(negative): missing team should rollback avatar and prom state.
// reserve_avatar(reserve_avatar)(negative): put URL failure should propagate after transaction commit.
// reserve_avatar(reserve_avatar)(negative): configured team avatar limit should reject oversized uploads.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): matching version should mark the team avatar uploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): repeated matching version confirmation should remain successful.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): stale version should leave avatar unuploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): old reservation replay should fail without marking current avatar uploaded.
// delete(delete)(positive): delete should mark a team invisible before background cleanup.
// delete(delete)(positive): sweeping an eligible team should enqueue its avatar deletion.
// delete(delete)(negative): missing team should rollback state.

use super::*;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use time::OffsetDateTime;

use crate::data::instr::team::{
    CreateTeamInstr, ListTeamInfosInstr, MarkTeamAvatarUploadedInstr,
    UpdateTeamInfoInstr,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockObjRecord};
use crate::result::ExpectedVariant;
use crate::test_util::fixture::team;
use crate::test_util::{assert_expected_message, assert_expected_variant};
use crate::usecase::team::read::{get_info, list_infos};
use crate::value::role::{RoleField, RoleMask};

// Build a team fixture with explicit avatar metadata for avatar-related assertions.
// Build a generic member fixture for team-related membership checks.
fn member(id: &str, user_id: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::ADMIN),
    }
}

// Build a token fixture that carries only the user id under test.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn seed_team_avatar(mock: &Mock, version: u32) {
    let key = ObjKey {
        id: "team--00000000001".into(),
        ver: version,
        image: [
            "team_avatar/team--00000000001/",
            &version.to_string(),
            ".png",
        ]
        .concat(),
    };

    let meta = ObjMeta {
        key,
        is_avail: false,
        hash: vec![0; 32],
        ext: "png".into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("team_avatar")
        .or_default()
        .insert(
            "team--00000000001".into(),
            MockObjRecord {
                version,
                meta: Some(meta),
            },
        );
}

async fn mark_team_avatar(
    mock: &Mock,
    version: u32,
) -> crate::result::BaseRest<()> {
    mark_avatar_uploaded::<MockContext, _, _>(
        (mock, mock),
        token("user--00000000001"),
        "team--00000000001".into(),
        MarkTeamAvatarUploadedInstr { image_ver: version },
    )
    .await
}

// Build login-credential data for the seeded user.
fn credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

// Build a user fixture with configurable super-admin flag.
fn user(id: &str, is_sadmin: bool) -> UserInfo {
    //
    let time = OffsetDateTime::now_utc();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: id.into(),
        is_sadmin,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

// Build list parameters for team pagination and optional owner filter.
fn list_instr(
    user_id: Option<&str>,
    offset: u32,
    limit: u32,
) -> ListTeamInfosInstr {
    ListTeamInfosInstr {
        user_id: user_id.map(Into::into),
        offset,
        limit: crate::value::pagination::PubListLimit::new(limit).unwrap(),
    }
}

// Build update instr carrying new team name and description.
fn update_instr(
    id: &str,
    name: &str,
    description: &str,
) -> UpdateTeamInfoInstr {
    UpdateTeamInfoInstr {
        id: id.into(),
        name: name.into(),
        description: description.into(),
    }
}

#[tokio::test]
async fn create_persists_team_and_returns_info() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user--00000000001", true),
        credential("user--00000000001"),
    );

    let val = create(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        CreateTeamInstr {
            name: "\u{2003}Team\u{00a0}".into(),
            description: "\u{2003}Desc\u{00a0}".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(val.name, "Team");

    assert_eq!(val.description, "Desc");

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.teams.len(), 1);

    assert_eq!(snapshot.teams.first().unwrap().id, val.id);

    assert_eq!(snapshot.teams.first().unwrap().name, "Team");

    assert_eq!(snapshot.teams.first().unwrap().description, "Desc");
}

#[tokio::test]
async fn create_makes_creator_admin_member() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user--00000000001", true),
        credential("user--00000000001"),
    );

    let val = create(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        CreateTeamInstr {
            name: "Team".into(),
            description: "Desc".into(),
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    let member_info = snapshot
        .members
        .iter()
        .find(|member_info| member_info.team_id == val.id)
        .unwrap();

    assert_eq!(member_info.user_id, "user--00000000001");

    assert!(member_info.roles.has_any_role(&[RoleField::ADMIN]));
}

#[tokio::test]
async fn create_propagates_repo_failure() {
    //
    let mock = Mock::new().with_create_team_failure();

    mock.seed_user(
        user("user--00000000001", true),
        credential("user--00000000001"),
    );

    let error = create(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        CreateTeamInstr {
            name: "Team".into(),
            description: "Desc".into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}

#[tokio::test]
async fn get_info_returns_team_without_avatar() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team--00000000001", "Team", "Desc"));

    let val = get_info((&mock, &mock), "team--00000000001".into())
        .await
        .unwrap();

    assert_eq!(val.id, "team--00000000001");

    assert!(val.avatar_url.is_none());
}

#[tokio::test]
async fn get_info_propagates_missing_team() {
    //
    let mock = Mock::new();

    let error = get_info((&mock, &mock), "team--00000000001".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_returns_paged_teams() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team--00000000001", "A", "Desc"));

    mock.seed_team(team("team--00000000002", "B", "Desc"));

    mock.seed_team(team("team--00000000003", "C", "Desc"));

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "team--00000000002",
    ));

    mock.seed_member(member(
        "member--00000000002",
        "user--00000000001",
        "team--00000000003",
    ));

    mock.seed_member(member(
        "member--00000000003",
        "user--00000000002",
        "team--00000000001",
    ));

    let val = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        list_instr(Some("user--00000000001"), 0, 1),
    )
    .await
    .unwrap();

    assert_eq!(val.len(), 1);

    assert_ne!(val.first().unwrap().id, "team--00000000001");
}

#[tokio::test]
async fn list_infos_returns_empty_page_source_when_offset_exceeds_data() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team--00000000001", "A", "Desc"));

    let val = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        list_instr(Some("user--00000000001"), 10, 10),
    )
    .await
    .unwrap();

    assert!(val.is_empty());
}

#[tokio::test]
async fn list_infos_all_teams_requires_sadmin() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team--00000000001", "A", "Desc"));

    mock.seed_user(
        user("user--00000000001", false),
        credential("user--00000000001"),
    );

    let error = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        list_instr(None, 0, 10),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}

#[tokio::test]
async fn list_infos_rejects_another_user_filter() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team--00000000001", "A", "Desc"));

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000002",
        "team--00000000001",
    ));

    let error = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        list_instr(Some("user--00000000002"), 0, 10),
    )
    .await
    .err()
    .unwrap();

    assert_expected_message(error, ExpectedVariant::Perm, "error-forbidden");
}

#[tokio::test]
async fn update_info_updates_team() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team--00000000001", "Old", "Old Desc"));

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "team--00000000001",
    ));

    update_info(
        (&mock,),
        token("user--00000000001"),
        update_instr(
            "team--00000000001",
            "\u{2003}New\u{00a0}",
            "\u{2003}New Desc\u{00a0}",
        ),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.teams.first().unwrap().name, "New");

    assert_eq!(snapshot.teams.first().unwrap().description, "New Desc");
}

#[tokio::test]
async fn update_info_propagates_missing_team() {
    //
    let mock = Mock::new();

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "team--00000000001",
    ));

    let error = update_info(
        (&mock,),
        token("user--00000000001"),
        update_instr("team--00000000001", "New", "New Desc"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn mark_avatar_uploaded_marks_current_generation_idempotently() {
    let mock = Mock::new();

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "team--00000000001",
    ));

    seed_team_avatar(&mock, 3);

    mark_team_avatar(&mock, 3).await.unwrap();

    mark_team_avatar(&mock, 3).await.unwrap();

    assert!(
        mock.snapshot()
            .objs
            .get("team_avatar")
            .unwrap()
            .get("team--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_stale_generation() {
    let mock = Mock::new();

    mock.seed_member(member(
        "member--00000000001",
        "user--00000000001",
        "team--00000000001",
    ));

    seed_team_avatar(&mock, 3);

    let error = mark_team_avatar(&mock, 2).await.err().unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(
        !mock
            .snapshot()
            .objs
            .get("team_avatar")
            .unwrap()
            .get("team--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}
