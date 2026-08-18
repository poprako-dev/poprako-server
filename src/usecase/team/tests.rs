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
// update_info(update_info)(positive): existing team should update name and description.
// update_info(update_info)(negative): missing team should propagate an argument error.
// reserve_avatar(reserve_avatar)(positive): first reservation should update avatar state, enqueue a check, and return a put URL.
// reserve_avatar(reserve_avatar)(positive): replacing an avatar should enqueue delete and check messages.
// reserve_avatar(reserve_avatar)(negative): missing team should rollback avatar and prom state.
// reserve_avatar(reserve_avatar)(negative): put URL failure should propagate after transaction commit.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): matching version should mark the team avatar uploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): repeated matching version confirmation should remain successful.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): stale version should leave avatar unuploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): old reservation replay should fail without marking current avatar uploaded.
// delete(delete)(positive): delete should remove team, worksets, descendant comics, and enqueue uploaded avatar deletion.
// delete(delete)(positive): deleting a team without uploaded avatar should not enqueue prom records.
// delete(delete)(negative): missing team should rollback state.

use super::*;

use time::OffsetDateTime;

use crate::data::instr::team::{
    CreateTeamInstr, ListTeamInfosInstr, MarkTeamAvatarUploadedInstr,
    ReserveTeamAvatarInstr, UpdateTeamInfoInstr,
};
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::shared::user::UserToken;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::image::{ImagePayload, ResourceKind};
use crate::part_impl::prom::mock_impl::MockPromRecord;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::fixture::{team, workset};
use crate::test_util::{
    assert_expected_message, assert_expected_variant,
    assert_one_image_check_record,
};
use crate::usecase::team::delete::delete;
use crate::usecase::team::read::{get_info, list_infos};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

// Build a team fixture with explicit avatar metadata for avatar-related assertions.
fn team_with_avatar(
    id: &str,
    name: &str,
    description: &str,
    avatar_key: &str,
    avatar_uploaded: bool,
    avatar_version: u32,
) -> TeamInfo {
    TeamInfo {
        avatar_key: Some(avatar_key.into()),
        is_avatar_uploaded: Some(avatar_uploaded),
        avatar_version: Some(avatar_version),
        ..team(id, name, description)
    }
}

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

// Build a comic fixture that already has an uploaded cover.
fn comic_with_uploaded_cover(
    id: &str,
    workset_id: &str,
    cover_key: &str,
) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        cover_key: Some(cover_key.into()),
        is_cover_uploaded: Some(true),
        cover_version: Some(1),
        cover_hash: Some(ImageHash::default()),
        cover_ext: Some(ImageExt::Png),
        chapter_count: 0,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        archived_at: None,
        created_at: time,
        updated_at: time,
    }
}

// Build a token fixture that carries only the user id under test.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
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
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
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
        limit,
    }
}

// Build avatar-reservation instr with fixed byte length and hash.
fn reserve_instr(file_ext: &str) -> ReserveTeamAvatarInstr {
    ReserveTeamAvatarInstr {
        image_hash: ImageHash::new([1; 32]),
        new_byte_len: 4096,
        ext: ImageExt::parse(file_ext).unwrap(),
    }
}

// Build mark-upload instr targeting a specific cover version.
fn mark_instr(image_version: u32) -> MarkTeamAvatarUploadedInstr {
    MarkTeamAvatarUploadedInstr { image_version }
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

// Count deferred image-delete prom records for a specific object key.
fn count_delete_records(records: &[MockPromRecord], object_key: &str) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                record.payload(),
                TaskPayload::Image {
                    payload: ImagePayload::Delete { object_key: key },
                }
                    if key == object_key
            )
        })
        .count()
}

#[tokio::test]
async fn create_persists_team_and_returns_info() {
    //
    let mock = Mock::new();

    mock.seed_user(user("user-1", true), credential("user-1"));

    let val = create(
        (&mock, &mock, &mock),
        token("user-1"),
        CreateTeamInstr {
            name: "Team".into(),
            description: "Desc".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(val.name, "Team");

    assert_eq!(val.description, "Desc");

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.teams.len(), 1);

    assert_eq!(snapshot.teams[0].id, val.id);
}

#[tokio::test]
async fn create_makes_creator_admin_member() {
    //
    let mock = Mock::new();

    mock.seed_user(user("user-1", true), credential("user-1"));

    let val = create(
        (&mock, &mock, &mock),
        token("user-1"),
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

    assert_eq!(member_info.user_id, "user-1");

    assert!(member_info.roles.has_any_role(&[RoleField::ADMIN]));
}

#[tokio::test]
async fn create_propagates_repo_failure() {
    //
    let mock = Mock::new().with_create_team_failure();

    mock.seed_user(user("user-1", true), credential("user-1"));

    let err = create(
        (&mock, &mock, &mock),
        token("user-1"),
        CreateTeamInstr {
            name: "Team".into(),
            description: "Desc".into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn get_info_returns_uploaded_avatar_url() {
    //
    let mock = Mock::new();

    mock.seed_team(team_with_avatar(
        "team-1",
        "Team",
        "Desc",
        "avatar-key",
        true,
        2,
    ));

    let val = get_info((&mock, &mock), "team-1".into()).await.unwrap();

    assert_eq!(val.id, "team-1");

    assert_eq!(
        val.avatar_url.as_deref(),
        Some("https://test.local/get/avatar-key")
    );

    assert_eq!(
        val.avatar_thumbnail_url.as_deref(),
        Some(
            "https://test.local/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/avatar-key"
        )
    );
}

#[tokio::test]
async fn get_info_propagates_missing_team() {
    //
    let mock = Mock::new();

    let err = get_info((&mock, &mock), "team-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_returns_paged_teams() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "A", "Desc"));

    mock.seed_team(team("team-2", "B", "Desc"));

    mock.seed_team(team("team-3", "C", "Desc"));

    mock.seed_member(member("member-1", "user-1", "team-2"));

    mock.seed_member(member("member-2", "user-1", "team-3"));

    mock.seed_member(member("member-3", "user-2", "team-1"));

    let val = list_infos(
        (&mock, &mock),
        token("user-1"),
        list_instr(Some("user-1"), 0, 1),
    )
    .await
    .unwrap();

    assert_eq!(val.len(), 1);

    assert_ne!(val[0].id, "team-1");
}

#[tokio::test]
async fn list_infos_returns_empty_page_when_offset_exceeds_data() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "A", "Desc"));

    let val = list_infos(
        (&mock, &mock),
        token("user-1"),
        list_instr(Some("user-1"), 10, 10),
    )
    .await
    .unwrap();

    assert!(val.is_empty());
}

#[tokio::test]
async fn list_infos_all_teams_requires_sadmin() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "A", "Desc"));

    mock.seed_user(user("user-1", false), credential("user-1"));

    let err =
        list_infos((&mock, &mock), token("user-1"), list_instr(None, 0, 10))
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_info_updates_team() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Old", "Old Desc"));

    mock.seed_member(member("member-1", "user-1", "team-1"));

    update_info(
        (&mock,),
        token("user-1"),
        update_instr("team-1", "New", "New Desc"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.teams[0].name, "New");

    assert_eq!(snapshot.teams[0].description, "New Desc");
}

#[tokio::test]
async fn update_info_propagates_missing_team() {
    //
    let mock = Mock::new();

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let err = update_info(
        (&mock,),
        token("user-1"),
        update_instr("team-1", "New", "New Desc"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}
