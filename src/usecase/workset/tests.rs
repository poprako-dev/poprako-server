// create(create)(positive): creating a workset should allocate team-scoped index and persist it.
// create(create)(negative): missing team should rollback without creating a workset.
// get_info(get_info)(positive): existing workset should return presentation-ready info.
// get_info(get_info)(negative): missing workset should propagate an argument error.
// list_infos(list_infos)(positive): list should return team worksets sorted by index.
// list_infos(list_infos)(positive): empty contents should return an empty list after membership.
// update_info(update_info)(positive): existing workset should update name and description.
// update_info(update_info)(negative): missing workset should propagate an argument error.
// delete(delete)(positive): deleting a workset with covered comics should enqueue cover deletions.
// delete(delete)(positive): direct repo delete should not create prom records.
// delete(delete)(negative): missing workset should rollback state.

use super::*;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use time::OffsetDateTime;

use crate::model::member::MemberInfo;
use crate::model::role::{RoleField, RoleMask};
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::part::prom::intention::ImageIntention;
use crate::part::prom::{Payload, PromStep};
use crate::part_impl::prom_mock::MockPromRecord;
use crate::part_impl::repo_mock::{Mock, MockTransactional};
use crate::result::ExpectedVariant;
use crate::result::accept;
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

fn create_data(team_id: &str) -> CreateWorksetData {
    CreateWorksetData {
        team_id: team_id.into(),
        name: "new".into(),
        description: Some("desc".into()),
    }
}

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn admin_member(user_id: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}-{}", user_id, team_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        team_id: team_id.into(),
        roles: RoleMask::from(RoleField::ADMIN),
    }
}

fn comic_with_uploaded_cover(
    id: &str,
    workset_id: &str,
    cover_key: &str,
) -> crate::model::comic::ComicInfo {
    let time = OffsetDateTime::now_utc();

    crate::model::comic::ComicInfo {
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

#[tokio::test]
async fn create_allocates_index_and_persists() {
    let mock = Mock::new();
    mock.seed_team(team("team-1", "Team", "Desc"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let created = create(&mock, &mock, token("user-1"), create_data("team-1")).await.unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(created.id, snapshot.worksets[0].id);
    assert_eq!(snapshot.worksets[0].index, 0);
    assert_eq!(snapshot.teams[0].workset_next_index, 1);
    assert_eq!(snapshot.worksets.len(), 1);
    assert_eq!(snapshot.worksets[0].name, "new");
}

#[tokio::test]
async fn create_rolls_back_missing_team() {
    let mock = Mock::new();
    mock.seed_member(admin_member("user-1", "missing"));

    let err = create(&mock, &mock, token("user-1"), create_data("missing"))
        .await
        .err()
        .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::ArgsInvalid);
    assert!(snapshot.worksets.is_empty());
}

#[tokio::test]
async fn get_info_returns_existing_workset() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 2));
    mock.seed_member(admin_member("user-1", "team-1"));

    let found = get_info(&mock, token("user-1"), "workset-1".into()).await.unwrap();

    assert_eq!(found.id, "workset-1");
    assert_eq!(found.index, 2);
}

#[tokio::test]
async fn get_info_propagates_missing_workset() {
    let mock = Mock::new();

    let err = get_info(&mock, token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::ArgsInvalid);
}

#[tokio::test]
async fn list_infos_filters_and_sorts_by_index() {
    let mock = Mock::new();
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_workset(workset("workset-2", "team-1", 2));
    mock.seed_workset(workset("workset-1", "team-1", 1));
    mock.seed_workset(workset("workset-other", "team-2", 0));

    let list = list_infos(
        &mock,
        token("user-1"),
        ListWorksetInfosData {
            team_id: "team-1".into(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .unwrap();

    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "workset-1");
    assert_eq!(list[1].id, "workset-2");
}

#[tokio::test]
async fn list_infos_returns_empty_for_missing_team_contents() {
    let mock = Mock::new();
    mock.seed_member(admin_member("user-1", "missing"));

    let list = list_infos(
        &mock,
        token("user-1"),
        ListWorksetInfosData {
            team_id: "missing".into(),
            offset: 0,
            limit: 10,
        },
    )
    .await
    .unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn update_info_updates_workset() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 0));
    mock.seed_member(admin_member("user-1", "team-1"));

    update_info(
        &mock,
        token("user-1"),
        UpdateWorksetInfoData {
            id: "workset-1".into(),
            name: "updated".into(),
            description: Some("updated-desc".into()),
        },
    )
    .await
    .unwrap();
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
        token("user-1"),
        UpdateWorksetInfoData {
            id: "missing".into(),
            name: "updated".into(),
            description: None,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::ArgsInvalid);
}

#[tokio::test]
async fn delete_removes_workset_and_enqueues_child_cover_deletes() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 0));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic_with_uploaded_cover(
        "comic-1",
        "workset-1",
        "cover-1.png",
    ));
    mock.seed_comic(comic_with_uploaded_cover(
        "comic-2",
        "workset-1",
        "cover-2.png",
    ));

    delete(&mock, &mock, &mock, token("user-1"), "workset-1".into()).await.unwrap();
    let snapshot = mock.snapshot();

    assert!(snapshot.worksets.is_empty());
    assert!(snapshot.comics.is_empty());
    assert_eq!(
        count_delete_records(&snapshot.prom_records, "cover-1.png"),
        1
    );
    assert_eq!(
        count_delete_records(&snapshot.prom_records, "cover-2.png"),
        1
    );
}

#[tokio::test]
async fn delete_rolls_back_missing_workset() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 0));
    mock.seed_member(admin_member("user-1", "team-1"));

    let err = delete(&mock, &mock, &mock, token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();
    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::ArgsInvalid);
    assert_eq!(snapshot.worksets.len(), 1);
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn delete_does_not_create_prom_records_when_called_directly() {
    let mock = Mock::new();
    mock.seed_workset(workset("workset-1", "team-1", 0));

    Drive::with_context(&mock, async move |context| {
        let transactional = MockTransactional;

        Advance::advance(
            &transactional,
            context,
            &PromStep::append(
                "prom-1",
                "image",
                Payload::Image(ImageIntention::Delete {
                    object_key: "existing.png".into(),
                }),
                &OffsetDateTime::now_utc(),
            ),
        )
        .await?;

        Advance::advance(
            &transactional,
            context,
            &crate::part::repo::step::workset::WorksetStep::delete("workset-1"),
        )
        .await?;

        accept(())
    })
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();
    assert_eq!(
        count_delete_records(&snapshot.prom_records, "existing.png"),
        1
    );
    assert_eq!(snapshot.prom_records.len(), 1);
    assert!(snapshot.worksets.is_empty());
}
