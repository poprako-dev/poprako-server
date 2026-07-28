// create(create)(positive): creating a workset should allocate team-scoped index and persist it.
// create(create)(negative): missing team should rollback without creating a workset.
// get_info(get_info)(positive): existing workset should return presentation-ready info.
// get_info(get_info)(negative): missing workset should propagate an argument error.
// list_infos(list_infos)(positive): list should return team worksets sorted by index.
// list_infos(list_infos)(positive): empty contents should return an empty list after membership.
// update_info(update_info)(positive): existing workset should update name and description.
// update_info(update_info)(negative): missing workset should propagate an argument error.
// delete(delete)(positive): deleting a workset with covered comics should enqueue cover deletions.
// delete(delete)(positive): deleting more than one comic batch should enqueue every cover deletion.
// delete(delete)(positive): direct repo delete should not create prom records.
// delete(delete)(negative): missing workset should rollback state.

use super::*;
use crate::data::instr::workset::{
    CreateWorksetInstr, ListWorksetInfosInstr, UpdateWorksetInfoInstr,
};

use poprako_orchestra::Step as _;
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;

use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::oper::workset::DeleteWorkset;
use crate::part_impl::prom::mock_impl::MockPromRecord;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{ExpectedVariant, accept};
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::team;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

fn workset(id: &str, team_id: &str, index: i32) -> WorksetInfo {
    //
    // Build a basic workset fixture for pagination and mutation tests.
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index,
        name: format!("workset-{}", index),
        description: None,
        comic_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn create_instr(team_id: &str) -> CreateWorksetInstr {
    // Build create instr with stable name/description defaults.
    CreateWorksetInstr {
        team_id: team_id.into(),
        name: "new".into(),
        description: Some("desc".into()),
    }
}

fn token(user_id: &str) -> UserToken {
    // Build token fixture for workset API request context.
    UserToken {
        user_id: user_id.into(),
    }
}

fn admin_member(user_id: &str, team_id: &str) -> MemberInfo {
    // Build an admin member fixture for workset admin-only operations.
    MemberInfo {
        id: format!("member-{}-{}", user_id, team_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::ADMIN),
    }
}

fn comic_with_uploaded_cover(
    id: &str,
    workset_id: &str,
    cover_key: &str,
) -> ComicInfo {
    //
    // Build a comic fixture with pre-uploaded cover metadata for delete assertions.
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        cover_key: Some(cover_key.into()),
        is_cover_uploaded: true,
        cover_version: 1,
        cover_hash: ImageHash::default(),
        cover_ext: ImageExt::Png,
        chapter_count: 0,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
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
                record.payload(),
                TaskPayload::Image(image::ImagePayload::Delete { object_key: key })
                    if key == object_key
            )
        })
        .count()
}

#[tokio::test]
async fn create_allocates_index_and_persists() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(admin_member("user-1", "team-1"));

    let created =
        create((&mock, &mock), token("user-1"), create_instr("team-1"))
            .await
            .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(created.id, snapshot.worksets[0].id);

    assert_eq!(snapshot.worksets[0].index, 0);

    assert_eq!(snapshot.worksets.len(), 1);

    assert_eq!(snapshot.worksets[0].name, "new");
}

#[tokio::test]
async fn create_rolls_back_missing_team() {
    //
    let mock = Mock::new();

    mock.seed_member(admin_member("user-1", "missing"));

    let err = create((&mock, &mock), token("user-1"), create_instr("missing"))
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(snapshot.worksets.is_empty());
}

#[tokio::test]
async fn get_info_returns_existing_workset() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1", 2));

    mock.seed_member(admin_member("user-1", "team-1"));

    let found = get_info((&mock,), token("user-1"), "workset-1".into())
        .await
        .unwrap();

    assert_eq!(found.id, "workset-1");

    assert_eq!(found.index, 2);
}

#[tokio::test]
async fn get_info_propagates_missing_workset() {
    //
    let mock = Mock::new();

    let err = get_info((&mock,), token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_and_sorts_by_index() {
    //
    let mock = Mock::new();

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_workset(workset("workset-2", "team-1", 2));

    mock.seed_workset(workset("workset-1", "team-1", 1));

    mock.seed_workset(workset("workset-other", "team-2", 0));

    let list = list_infos(
        (&mock,),
        token("user-1"),
        ListWorksetInfosInstr {
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
    //
    let mock = Mock::new();

    mock.seed_member(admin_member("user-1", "missing"));

    let list = list_infos(
        (&mock,),
        token("user-1"),
        ListWorksetInfosInstr {
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
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1", 0));

    mock.seed_member(admin_member("user-1", "team-1"));

    update_info(
        (&mock,),
        token("user-1"),
        UpdateWorksetInfoInstr {
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
    //
    let mock = Mock::new();

    let err = update_info(
        (&mock,),
        token("user-1"),
        UpdateWorksetInfoInstr {
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
async fn delete_removes_workset_and_enqueues_child_cover_deletes() {
    //
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

    delete((&mock, &mock, &mock), token("user-1"), "workset-1".into())
        .await
        .unwrap();

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
async fn delete_enqueues_cover_deletes_across_multiple_comic_batches() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1", 0));

    mock.seed_member(admin_member("user-1", "team-1"));

    for comic_index in 0..=50 {
        //
        let comic_id = format!("comic-{}", comic_index);

        let cover_key = format!("cover-{}.png", comic_index);

        mock.seed_comic(comic_with_uploaded_cover(
            &comic_id,
            "workset-1",
            &cover_key,
        ));
    }

    delete((&mock, &mock, &mock), token("user-1"), "workset-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.worksets.is_empty());

    assert!(snapshot.comics.is_empty());

    assert_eq!(snapshot.prom_records.len(), 51);
}

#[tokio::test]
async fn delete_rolls_back_missing_workset() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1", 0));

    mock.seed_member(admin_member("user-1", "team-1"));

    let err = delete((&mock, &mock, &mock), token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(snapshot.worksets.len(), 1);

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn delete_does_not_create_prom_records_when_called_directly() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1", 0));

    mock.coord(async |context| {
        //
        let id = "prom-1".to_string();

        let payload = TaskPayload::Image(image::ImagePayload::Delete {
            object_key: "existing.png".to_string(),
        });

        let task = Task {
            id: &id,
            payload: &payload,
            delay: None,
        };

        mock.step(context, &Defer::new(task)).await?;

        mock.step(context, &DeleteWorkset { id: "workset-1" })
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
