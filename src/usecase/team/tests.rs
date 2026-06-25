//! Test fixtures and cases for the team use case module.
//!
//! Tests exercise team CRUD, avatar management, and deletion against
//! a [`Mock`] that doubles as the driver, repository, prom enqueuer,
//! and image pool. Negative cases use [`FailingCreateRepo`] to simulate
//! repository errors.
//!
//! [`Mock`]: crate::part_impl::repo_mock::Mock

// create(create)(positive): creating a team should persist it and return team info.
// create(create)(negative): create repo failure should propagate.
// get_info(get_info)(positive): existing team should return info with avatar URL when uploaded.
// get_info(get_info)(negative): missing team should propagate an argument error.
// list_infos(list_infos)(positive): list should return paged teams in repo order.
// list_infos(list_infos)(negative): missing page contents should return an empty list.
// update_info(update_info)(positive): existing team should update name and description.
// update_info(update_info)(negative): missing team should propagate an argument error.
// reserve_avatar(reserve_avatar)(positive): first reservation should update avatar state, enqueue a check, and return a put URL.
// reserve_avatar(reserve_avatar)(positive): replacing an avatar should enqueue delete and check messages.
// reserve_avatar(reserve_avatar)(negative): missing team should rollback avatar and prom state.
// reserve_avatar(reserve_avatar)(negative): put URL failure should propagate after transaction commit.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): matching version should mark the team avatar uploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): stale version should leave avatar unuploaded.
// delete(delete)(positive): delete should remove team, worksets, descendant comics, and enqueue uploaded avatar deletion.
// delete(delete)(positive): deleting a team without uploaded avatar should not enqueue prom records.
// delete(delete)(negative): missing team should rollback state.

use super::*;

use async_trait::async_trait;

use poprako_transactional::advance::Advance;
use poprako_util::page::Page;

use time::OffsetDateTime;

use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::workset::WorksetInfo;
use crate::part::prom::Payload;
use crate::part::prom::intention::{ImageIntention, ImageKind};
use crate::part::repo::Execute;
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrementWorksetNextIndex, List,
    MarkAvatarUploaded, ReserveAvatar, UpdateInfo,
};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part_impl::prom_mock::MockPromRecord;
use crate::part_impl::repo_mock::{Mock, MockContext};
use crate::result::{ExpectedVariant, RootError};
use crate::test_util::assert_expected_variant;
use crate::util::DeriveTransactional;

/// A repository whose [`Execute`] and [`Advance`] impls always fail.
///
/// Used in negative tests to verify error propagation from the repo layer.
/// Implements all [`TeamRepo`] operations by delegating to
/// [`FailingTeamTransactional`].
struct FailingCreateRepo;

#[async_trait]
impl DeriveTransactional for FailingCreateRepo {
    type Transactional = FailingTeamTransactional;

    async fn transactional(&self) -> Self::Transactional {
        FailingTeamTransactional
    }
}

impl TeamRepo<MockContext> for FailingCreateRepo {}

struct FailingTeamTransactional;

impl TeamRepoTransactional<MockContext> for FailingTeamTransactional {}

/// Builds a [`TeamInfo`] fixture with default timestamps and no avatar.
pub(crate) fn team(id: &str, name: &str, description: &str) -> TeamInfo {
    let time = OffsetDateTime::now_utc();

    TeamInfo {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        workset_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

/// Builds a [`TeamInfo`] fixture with avatar fields set.
pub(crate) fn team_with_avatar(
    id: &str,
    name: &str,
    description: &str,
    avatar_key: &str,
    avatar_uploaded: bool,
    avatar_version: i64,
) -> TeamInfo {
    TeamInfo {
        avatar_key: Some(avatar_key.into()),
        avatar_uploaded,
        avatar_version,
        ..team(id, name, description)
    }
}

/// Builds a [`WorksetInfo`] fixture.
pub(crate) fn workset(id: &str, team_id: &str) -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 0,
        comic_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

/// Builds a [`ComicInfo`] fixture with an uploaded cover.
fn comic_with_uploaded_cover(id: &str, workset_id: &str, cover_key: &str) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        is_completed: false,
        cover_key: Some(cover_key.into()),
        cover_uploaded: true,
        cover_version: 1,
        chapter_count: 0,
        chapter_next_index: 0,
        creator_id: "user-1".into(),
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

/// Builds a standard error for negative test cases.
fn expected_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::Args,
        message: "failed".into(),
    }
}

/// Builds a [`Page`] with given offset and limit.
fn page(offset: usize, limit: usize) -> Page {
    Page { offset, limit }
}

/// Builds a [`ReserveTeamAvatarData`] fixture.
fn reserve_data(file_ext: &str) -> ReserveTeamAvatarData {
    ReserveTeamAvatarData {
        file_ext: file_ext.into(),
    }
}

/// Builds a [`MarkTeamAvatarUploadedData`] fixture.
fn mark_data(avatar_version: i64) -> MarkTeamAvatarUploadedData {
    MarkTeamAvatarUploadedData { avatar_version }
}

/// Builds an [`UpdateTeamInfoData`] fixture.
fn update_data(id: &str, name: &str, description: &str) -> UpdateTeamInfoData {
    UpdateTeamInfoData {
        id: id.into(),
        name: name.into(),
        description: description.into(),
    }
}

/// Counts [`Delete`](ImageIntention::Delete) prom records matching the given object key.
fn count_delete_records(records: &[MockPromRecord], object_key: &str) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                &record.payload,
                Payload::Image(ImageIntention::Delete { object_key: key })
                    if key == object_key
            )
        })
        .count()
}

/// Counts [`CheckUploaded`](ImageIntention::CheckUploaded) prom records for team avatars.
fn count_team_check_records(
    records: &[MockPromRecord],
    resource_id: &str,
    object_key: &str,
    image_version: i64,
) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                &record.payload,
                Payload::Image(ImageIntention::CheckUploaded {
                    kind: ImageKind::TeamAvatar,
                    resource_id: id,
                    object_key: key,
                    image_version: version,
                }) if id == resource_id && key == object_key && *version == image_version
            )
        })
        .count()
}

#[async_trait]
impl<'a> Execute<Create<'a>> for FailingCreateRepo {
    type Error = RootError;

    async fn execute(&self, _step: &Create<'a>) -> Result<TeamInfo, Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for FailingCreateRepo {
    type Error = RootError;

    async fn execute(&self, _step: &GetInfoById<'a>) -> Result<TeamInfo, Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl Execute<List> for FailingCreateRepo {
    type Error = RootError;

    async fn execute(&self, _step: &List) -> Result<Vec<TeamInfo>, Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for FailingCreateRepo {
    type Error = RootError;

    async fn execute(&self, _step: &UpdateInfo<'a>) -> Result<(), Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Execute<MarkAvatarUploaded<'a>> for FailingCreateRepo {
    type Error = RootError;

    async fn execute(&self, _step: &MarkAvatarUploaded<'a>) -> Result<(), Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, MockContext> for FailingTeamTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        _context: &mut MockContext,
        _step: &ReserveAvatar<'a>,
    ) -> Result<crate::model::team::TeamAvatarReservation, Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, MockContext> for FailingTeamTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        _context: &mut MockContext,
        _step: &MarkAvatarUploaded<'a>,
    ) -> Result<(), Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for FailingTeamTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        _context: &mut MockContext,
        _step: &GetInfoExcluded<'a>,
    ) -> Result<TeamInfo, Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, MockContext> for FailingTeamTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        _context: &mut MockContext,
        _step: &Delete<'a>,
    ) -> Result<(), Self::Error> {
        Err(expected_error())
    }
}

#[async_trait]
impl<'a> Advance<IncrementWorksetNextIndex<'a>, MockContext> for FailingTeamTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        _context: &mut MockContext,
        _step: &IncrementWorksetNextIndex<'a>,
    ) -> Result<i32, Self::Error> {
        Err(expected_error())
    }
}

#[tokio::test]
async fn create_persists_team_and_returns_info() {
    let mock = Mock::new();

    let result = create(
        &mock,
        &mock,
        CreateTeamData {
            name: "Team".into(),
            description: "Desc".into(),
        },
    )
    .await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert_eq!(result.name, "Team");
    assert_eq!(result.description, "Desc");
    let snapshot = mock.snapshot();
    assert_eq!(snapshot.teams.len(), 1);
    assert_eq!(snapshot.teams[0].id, result.id);
}

#[tokio::test]
async fn create_propagates_repo_failure() {
    let repo = FailingCreateRepo;
    let image = Mock::new();

    let err = create(
        &repo,
        &image,
        CreateTeamData {
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
    let mock = Mock::new();
    mock.seed_team(team_with_avatar(
        "team-1",
        "Team",
        "Desc",
        "avatar-key",
        true,
        2,
    ));

    let result = get_info(&mock, &mock, "team-1".into()).await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert_eq!(result.id, "team-1");
    assert_eq!(
        result.avatar_url.as_deref(),
        Some("https://test.local/get/avatar-key")
    );
}

#[tokio::test]
async fn get_info_propagates_missing_team() {
    let mock = Mock::new();

    let err = get_info(&mock, &mock, "team-1".into()).await.err().unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_returns_paged_teams() {
    let mock = Mock::new();
    mock.seed_team(team("team-1", "A", "Desc"));
    mock.seed_team(team("team-2", "B", "Desc"));

    let result = list_infos(&mock, &mock, page(0, 1)).await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert_eq!(result.len(), 1);
}

#[tokio::test]
async fn list_infos_returns_empty_page_when_offset_exceeds_data() {
    let mock = Mock::new();
    mock.seed_team(team("team-1", "A", "Desc"));

    let result = list_infos(&mock, &mock, page(10, 10)).await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn update_info_updates_team() {
    let mock = Mock::new();
    mock.seed_team(team("team-1", "Old", "Old Desc"));

    let result = update_info(&mock, update_data("team-1", "New", "New Desc")).await;
    assert!(result.is_ok());

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.teams[0].name, "New");
    assert_eq!(snapshot.teams[0].description, "New Desc");
}

#[tokio::test]
async fn update_info_propagates_missing_team() {
    let mock = Mock::new();

    let err = update_info(&mock, update_data("team-1", "New", "New Desc"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn reserve_avatar_updates_state_enqueues_check_and_returns_put_url() {
    let mock = Mock::new();
    mock.seed_team(team("team-1", "Team", "Desc"));

    let result = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        "team-1".into(),
        reserve_data("png"),
    )
    .await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert_eq!(result.avatar_version, 1);
    assert_eq!(
        result.put_url,
        "https://test.local/put/team_avatar/team-1-1.png"
    );

    let snapshot = mock.snapshot();
    assert_eq!(
        snapshot.teams[0].avatar_key.as_deref(),
        Some("team_avatar/team-1-1.png")
    );
    assert_eq!(
        count_team_check_records(
            &snapshot.prom_records,
            "team-1",
            "team_avatar/team-1-1.png",
            1
        ),
        1
    );
}

#[tokio::test]
async fn reserve_avatar_replacing_avatar_enqueues_delete_and_check() {
    let mock = Mock::new();
    mock.seed_team(team_with_avatar(
        "team-1", "Team", "Desc", "old-key", true, 1,
    ));

    let result = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        "team-1".into(),
        reserve_data("jpg"),
    )
    .await;
    assert!(result.is_ok());

    let snapshot = mock.snapshot();
    assert_eq!(count_delete_records(&snapshot.prom_records, "old-key"), 1);
    assert_eq!(
        count_team_check_records(
            &snapshot.prom_records,
            "team-1",
            "team_avatar/team-1-2.jpg",
            2
        ),
        1
    );
}

#[tokio::test]
async fn reserve_avatar_rolls_back_missing_team() {
    let mock = Mock::new();

    let err = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        "team-1".into(),
        reserve_data("png"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    let snapshot = mock.snapshot();
    assert!(snapshot.teams.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn reserve_avatar_propagates_put_url_failure_after_commit() {
    let mock = Mock::new().with_image_put_failure();
    mock.seed_team(team("team-1", "Team", "Desc"));

    let err = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        "team-1".into(),
        reserve_data("png"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    let snapshot = mock.snapshot();
    assert_eq!(
        snapshot.teams[0].avatar_key.as_deref(),
        Some("team_avatar/team-1-1.png")
    );
    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn mark_avatar_uploaded_marks_matching_version() {
    let mock = Mock::new();
    mock.seed_team(team_with_avatar("team-1", "Team", "Desc", "key", false, 2));

    let result = mark_avatar_uploaded(&mock, "team-1".into(), mark_data(2)).await;
    assert!(result.is_ok());

    assert!(mock.snapshot().teams[0].avatar_uploaded);
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_stale_version() {
    let mock = Mock::new();
    mock.seed_team(team_with_avatar("team-1", "Team", "Desc", "key", false, 2));

    let err = mark_avatar_uploaded(&mock, "team-1".into(), mark_data(1))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(!mock.snapshot().teams[0].avatar_uploaded);
}

#[tokio::test]
async fn delete_removes_team_worksets_descendant_comics_and_avatar() {
    let mock = Mock::new();
    mock.seed_team(team_with_avatar(
        "team-1",
        "Team",
        "Desc",
        "avatar-key",
        true,
        2,
    ));
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_workset(workset("workset-2", "team-1"));
    mock.seed_comic(comic_with_uploaded_cover(
        "comic-1",
        "workset-1",
        "cover-1.png",
    ));
    mock.seed_comic(comic_with_uploaded_cover(
        "comic-2",
        "workset-2",
        "cover-2.png",
    ));

    let result = delete(&mock, &mock, &mock, "team-1".into()).await;
    assert!(result.is_ok());

    let snapshot = mock.snapshot();
    assert!(snapshot.teams.is_empty());
    assert!(snapshot.worksets.is_empty());
    assert!(snapshot.comics.is_empty());
    assert_eq!(count_delete_records(&snapshot.prom_records, "cover-1.png"), 1);
    assert_eq!(count_delete_records(&snapshot.prom_records, "cover-2.png"), 1);
    assert_eq!(
        count_delete_records(&snapshot.prom_records, "avatar-key"),
        1
    );
}

#[tokio::test]
async fn delete_without_uploaded_avatar_does_not_enqueue_prom() {
    let mock = Mock::new();
    mock.seed_team(team_with_avatar(
        "team-1",
        "Team",
        "Desc",
        "avatar-key",
        false,
        2,
    ));

    let result = delete(&mock, &mock, &mock, "team-1".into()).await;
    assert!(result.is_ok());

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn delete_rolls_back_missing_team() {
    let mock = Mock::new();

    let err = delete(&mock, &mock, &mock, "team-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().teams.is_empty());
    assert!(mock.snapshot().prom_records.is_empty());
}
