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

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::task::ObjTask;
use poprako_orchestra::{Nucl as _, OperStep as _};
use time::OffsetDateTime;

use crate::data::instr::workset::{
    CreateWorksetInstr, ListWorksetInfosInstr, UpdateWorksetInfoInstr,
};
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part::repo::oper::workset::DeleteWorkset;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};
use crate::result::{ExpectedVariant, accept};
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::team;
use crate::value::role::{RoleField, RoleMask};

fn workset(id: &str, team_id: &str, index: usize) -> WorksetInfo {
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

fn workset_with_comic_count(
    id: &str,
    team_id: &str,
    index: usize,
    comic_count: usize,
) -> WorksetInfo {
    let mut workset_info = workset(id, team_id, index);

    workset_info.comic_count = comic_count;

    workset_info
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

fn comic(id: &str, workset_id: &str, index: usize) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index,
        title: format!("comic-{index}"),
        author: "author".into(),
        description: None,
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

fn seed_comic_cover(mock: &Mock, comic_id: &str) -> ObjKey {
    let key = ObjKey {
        id: comic_id.into(),
        ver: 1,
        image: format!("comic_cover/{comic_id}-1.png"),
    };

    let meta = ObjMeta {
        key: key.clone(),
        is_avail: true,
        hash: vec![1; 32],
        ext: "png".into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("comic_cover")
        .or_default()
        .insert(
            comic_id.into(),
            MockObjRecord {
                version: key.ver,
                meta: Some(meta),
            },
        );

    key
}

fn count_delete_tasks(
    tasks: &[(&'static str, ObjTask)],
    expected_key: &ObjKey,
) -> usize {
    tasks
        .iter()
        .filter(|(topic, task)| {
            *topic == "comic_cover"
                && matches!(task, ObjTask::Delete { key } if key == expected_key)
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

    mock.seed_workset(workset_with_comic_count("workset-1", "team-1", 0, 2));

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
    let mock = Mock::new();

    mock.seed_workset(workset_with_comic_count("workset-1", "team-1", 0, 2));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1", 0));
    mock.seed_comic(comic("comic-2", "workset-1", 1));

    let first_key = seed_comic_cover(&mock, "comic-1");
    let second_key = seed_comic_cover(&mock, "comic-2");

    delete((&mock, &mock, &mock), token("user-1"), "workset-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.worksets.is_empty());
    assert!(snapshot.comics.is_empty());
    assert!(snapshot.objs["comic_cover"].is_empty());
    assert_eq!(snapshot.obj_tasks.len(), 2);
    assert_eq!(count_delete_tasks(&snapshot.obj_tasks, &first_key), 1);
    assert_eq!(count_delete_tasks(&snapshot.obj_tasks, &second_key), 1);
}

#[tokio::test]
async fn delete_enqueues_cover_deletes_across_multiple_comic_batches() {
    let mock = Mock::new();

    mock.seed_workset(workset_with_comic_count("workset-1", "team-1", 0, 51));
    mock.seed_member(admin_member("user-1", "team-1"));

    let mut cover_keys = Vec::new();

    for comic_index in 0..=50 {
        let comic_id = format!("comic-{comic_index}");

        mock.seed_comic(comic(&comic_id, "workset-1", comic_index));

        cover_keys.push(seed_comic_cover(&mock, &comic_id));
    }

    delete((&mock, &mock, &mock), token("user-1"), "workset-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.worksets.is_empty());
    assert!(snapshot.comics.is_empty());
    assert!(snapshot.objs["comic_cover"].is_empty());
    assert_eq!(snapshot.obj_tasks.len(), 51);

    for cover_key in cover_keys {
        assert_eq!(count_delete_tasks(&snapshot.obj_tasks, &cover_key), 1);
    }
}

#[tokio::test]
async fn delete_rolls_back_missing_workset() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1", 0));
    mock.seed_member(admin_member("user-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1", 0));

    let cover_key = seed_comic_cover(&mock, "comic-1");

    let err = delete((&mock, &mock, &mock), token("user-1"), "missing".into())
        .await
        .err()
        .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(snapshot.worksets.len(), 1);
    assert_eq!(snapshot.comics.len(), 1);
    assert_eq!(
        snapshot.objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        cover_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn direct_repo_delete_does_not_mutate_obj_dept_state() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1", 0));

    let unrelated_key = seed_comic_cover(&mock, "comic-1");

    mock.coord(async |context| {
        DeleteWorkset { id: "workset-1" }
            .step_on(&mock, context)
            .await?;

        accept(())
    })
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.worksets.is_empty());
    assert_eq!(
        snapshot.objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        unrelated_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}
