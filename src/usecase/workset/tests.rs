// create(create)(positive): creating a workset should allocate team-scoped index and persist it.
// create(create)(negative): missing team should rollback without creating a workset.
// get_info(get_info)(positive): existing workset should return presentation-ready info.
// get_info(get_info)(negative): missing workset should propagate an argument error.
// list_infos(list_infos)(positive): list should return team worksets sorted by index.
// list_infos(list_infos)(negative): missing team contents should return an empty list.
// update_info(update_info)(positive): existing workset should update name and description.
// update_info(update_info)(negative): missing workset should propagate an argument error.
// delete(delete)(positive): existing workset should be removed.
// delete(delete)(negative): missing workset should rollback state.

use super::*;

use time::OffsetDateTime;

use crate::model::workset::WorksetInfo;
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::usecase::team::tests::team;

fn workset(id: &str, team_id: &str, index: i32) -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index,
        name: format!("workset-{}", index),
        description: None,
        comic_count: 0,
        comic_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

fn create_data(team_id: &str) -> WorksetCreateData {
    WorksetCreateData {
        team_id: team_id.into(),
        name: "new".into(),
        description: Some("desc".into()),
    }
}

#[tokio::test]
async fn create_allocates_index_and_persists() {
    let mock = Mock::new();
    mock.seed_team(team("team-1", "Team", "Desc"));

    let created = create(&mock, &mock, create_data("team-1")).await;
    assert!(created.is_ok());
    let created = created.ok().unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(created.workset.index, 0);
    assert_eq!(snapshot.teams[0].workset_next_index, 1);
    assert_eq!(snapshot.worksets.len(), 1);
    assert_eq!(snapshot.worksets[0].name, "new");
}

#[tokio::test]
async fn create_rolls_back_missing_team() {
    let mock = Mock::new();

    let err = create(&mock, &mock, create_data("missing"))
        .await
        .err()
        .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(snapshot.worksets.is_empty());
}

#[tokio::test]
async fn get_info_returns_existing_workset() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 2));

    let found = get_info(&mock, "workset-1".into()).await;
    assert!(found.is_ok());
    let found = found.ok().unwrap();

    assert_eq!(found.id, "workset-1");
    assert_eq!(found.index, 2);
}

#[tokio::test]
async fn get_info_propagates_missing_workset() {
    let mock = Mock::new();

    let err = get_info(&mock, "missing".into()).await.err().unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_and_sorts_by_index() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-2", "team-1", 2));
    mock.seed_workset(workset("workset-1", "team-1", 1));
    mock.seed_workset(workset("workset-other", "team-2", 0));

    let list = list_infos(
        &mock,
        WorksetListData {
            team_id: "team-1".into(),
        },
    )
    .await;
    assert!(list.is_ok());
    let list = list.ok().unwrap();

    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "workset-1");
    assert_eq!(list[1].id, "workset-2");
}

#[tokio::test]
async fn list_infos_returns_empty_for_missing_team_contents() {
    let mock = Mock::new();

    let list = list_infos(
        &mock,
        WorksetListData {
            team_id: "missing".into(),
        },
    )
    .await;
    assert!(list.is_ok());

    assert!(list.ok().unwrap().is_empty());
}

#[tokio::test]
async fn update_info_updates_workset() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 0));

    let result = update_info(
        &mock,
        WorksetInfoUpdateData {
            id: "workset-1".into(),
            name: "updated".into(),
            description: Some("updated-desc".into()),
        },
    )
    .await;
    assert!(result.is_ok());
    let snapshot = mock.snapshot();

    assert_eq!(snapshot.worksets[0].name, "updated");
    assert_eq!(
        snapshot.worksets[0].description,
        Some("updated-desc".into())
    );
}

#[tokio::test]
async fn update_info_propagates_missing_workset() {
    let mock = Mock::new();

    let err = update_info(
        &mock,
        WorksetInfoUpdateData {
            id: "missing".into(),
            name: "updated".into(),
            description: None,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn delete_removes_workset() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 0));

    let result = delete(&mock, &mock, "workset-1".into()).await;
    assert!(result.is_ok());

    assert!(mock.snapshot().worksets.is_empty());
}

#[tokio::test]
async fn delete_rolls_back_missing_workset() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 0));

    let err = delete(&mock, &mock, "missing".into()).await.err().unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(snapshot.worksets.len(), 1);
}
