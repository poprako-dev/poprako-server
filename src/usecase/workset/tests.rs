#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// create(create)(positive): creating a workset should allocate team-scoped index and persist it.
// create(create)(negative): missing team should rollback without creating a workset.
// get_info(get_info)(positive): existing workset should return presentation-ready info.
// get_info(get_info)(negative): missing workset should propagate an argument error.
// list_infos(list_infos)(positive): list should return team worksets sorted by index.
// list_infos(list_infos)(positive): empty contents should return an empty list after membership.
// update_info(update_info)(positive): existing workset should update name and description.
// update_info(update_info)(negative): missing workset should propagate an argument error.
// delete(delete)(positive): deleting a workset should tombstone its whole hierarchy atomically.
// delete(delete)(positive): marking a large workset should not create object tasks eagerly.
// delete(delete)(negative): missing workset should rollback state.

use super::*;

mod subtree_delete;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use time::OffsetDateTime;

use crate::data::instr::workset::{
    CreateWorksetInstr, ListWorksetInfosInstr, UpdateWorksetInfoInstr,
};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::team;
use crate::usecase::subtree_delete::sweep;
use crate::value::chapter::mask::StageMask;
use crate::value::role::{RoleField, RoleMask};
use crate::value::subtree_delete::SubtreeSweepLevel;

fn workset(id: &str, team_id: &str, index: usize) -> WorksetInfo {
    //
    // Build a basic workset fixture for pagination and mutation tests.
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index,
        name: ["workset-", &index.to_string()].concat(),
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
        id: ["member-", user_id, "-", team_id].concat(),
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
        creator_id: "user--00000000001".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        archived_at: None,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str, comic_id: &str) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        confirmed_artwork_ver: None,
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: false,
        index: 0,
        subtitle: "chapter".into(),
        page_source_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).unwrap(),
        creator_id: "user--00000000001".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn seed_comic_cover(mock: &Mock, comic_id: &str) -> ObjKey {
    let key = ObjKey {
        id: comic_id.into(),
        ver: 1,
        image: format!("comic_cover/{comic_id}/1.png"),
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

#[tokio::test]
async fn create_allocates_index_and_persists() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team--00000000001", "Team", "Desc"));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    let mut instr = create_instr("team--00000000001");

    instr.name = "\u{2003}new\u{00a0}".into();

    instr.description = Some("\u{2003}desc\u{00a0}".into());

    let created = create((&mock, &mock), token("user--00000000001"), instr)
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(created.id, snapshot.worksets.first().unwrap().id);

    assert_eq!(snapshot.worksets.first().unwrap().index, 0);

    assert_eq!(snapshot.worksets.len(), 1);

    assert_eq!(snapshot.worksets.first().unwrap().name, "new");

    assert_eq!(
        snapshot.worksets.first().unwrap().description,
        Some("desc".into())
    );
}

#[tokio::test]
async fn create_rolls_back_missing_team() {
    //
    let mock = Mock::new();

    mock.seed_member(admin_member("user--00000000001", "missing"));

    let error = create(
        (&mock, &mock),
        token("user--00000000001"),
        create_instr("missing"),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(snapshot.worksets.is_empty());
}

#[tokio::test]
async fn get_info_returns_existing_workset() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001", 2));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    let found = get_info(
        (&mock,),
        token("user--00000000001"),
        "workset--00000000001".into(),
    )
    .await
    .unwrap();

    assert_eq!(found.id, "workset--00000000001");

    assert_eq!(found.index, 2);
}

#[tokio::test]
async fn get_info_propagates_missing_workset() {
    //
    let mock = Mock::new();

    let error =
        get_info((&mock,), token("user--00000000001"), "missing".into())
            .await
            .err()
            .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_and_sorts_by_index() {
    //
    let mock = Mock::new();

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    mock.seed_workset(workset("workset--00000000002", "team--00000000001", 2));

    mock.seed_workset(workset("workset--00000000001", "team--00000000001", 1));

    mock.seed_workset(workset("workset--85117610840", "team--00000000002", 0));

    let list = list_infos(
        (&mock,),
        token("user--00000000001"),
        ListWorksetInfosInstr {
            team_id: "team--00000000001".into(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(list.len(), 2);

    assert_eq!(list.first().unwrap().id, "workset--00000000001");

    assert_eq!(list.get(1).unwrap().id, "workset--00000000002");
}

#[tokio::test]
async fn list_infos_returns_empty_for_missing_team_contents() {
    //
    let mock = Mock::new();

    mock.seed_member(admin_member("user--00000000001", "missing"));

    let list = list_infos(
        (&mock,),
        token("user--00000000001"),
        ListWorksetInfosInstr {
            team_id: "missing".into(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
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

    mock.seed_workset(workset_with_comic_count(
        "workset--00000000001",
        "team--00000000001",
        0,
        2,
    ));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    update_info(
        (&mock,),
        token("user--00000000001"),
        UpdateWorksetInfoInstr {
            id: "workset--00000000001".into(),
            name: "\u{2003}updated\u{00a0}".into(),
            description: Some("\u{2003}updated-desc\u{00a0}".into()),
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.worksets.first().unwrap().name, "updated");

    assert_eq!(
        snapshot.worksets.first().unwrap().description,
        Some("updated-desc".into())
    );
}

#[tokio::test]
async fn update_info_propagates_missing_workset() {
    //
    let mock = Mock::new();

    let error = update_info(
        (&mock,),
        token("user--00000000001"),
        UpdateWorksetInfoInstr {
            id: "missing".into(),
            name: "updated".into(),
            description: None,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn delete_marks_workset_hierarchy_without_eager_object_deletes() {
    let mock = Mock::new();

    mock.seed_workset(workset_with_comic_count(
        "workset--00000000001",
        "team--00000000001",
        0,
        2,
    ));
    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));
    mock.seed_comic(comic("comic--00000000001", "workset--00000000001", 0));
    mock.seed_comic(comic("comic--00000000002", "workset--00000000001", 1));
    mock.seed_chapter(chapter("chapter--00000000001", "comic--00000000001"));

    seed_comic_cover(&mock, "comic--00000000001");
    seed_comic_cover(&mock, "comic--00000000002");

    delete(
        (&mock, &mock),
        token("user--00000000001"),
        "workset--00000000001".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.worksets.len(), 1);
    assert_eq!(snapshot.comics.len(), 2);
    assert!(
        snapshot
            .deleted_workset_ids
            .contains("workset--00000000001")
    );
    assert!(snapshot.deleted_comic_ids.contains("comic--00000000001"));
    assert!(snapshot.deleted_comic_ids.contains("comic--00000000002"));
    assert!(
        snapshot
            .deleted_chapter_ids
            .contains("chapter--00000000001")
    );
    assert_eq!(snapshot.objs.get("comic_cover").unwrap().len(), 2);
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn delete_marks_large_workset_without_eager_object_work() {
    let mock = Mock::new();

    mock.seed_workset(workset_with_comic_count(
        "workset--00000000001",
        "team--00000000001",
        0,
        513,
    ));
    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    for comic_index in 0..=512 {
        let comic_id = format!("comic-{comic_index}");

        mock.seed_comic(comic(&comic_id, "workset--00000000001", comic_index));

        seed_comic_cover(&mock, &comic_id);
    }

    delete(
        (&mock, &mock),
        token("user--00000000001"),
        "workset--00000000001".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.worksets.len(), 1);
    assert_eq!(snapshot.comics.len(), 513);
    assert!(
        snapshot
            .deleted_workset_ids
            .contains("workset--00000000001")
    );
    assert_eq!(snapshot.deleted_comic_ids.len(), 513);
    assert_eq!(snapshot.objs.get("comic_cover").unwrap().len(), 513);
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn background_sweep_deletes_comics_before_workset() {
    let mock = Mock::new();

    mock.seed_workset(workset_with_comic_count(
        "workset--00000000001",
        "team--00000000001",
        0,
        2,
    ));
    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));
    mock.seed_comic(comic("comic--00000000001", "workset--00000000001", 0));
    mock.seed_comic(comic("comic--00000000002", "workset--00000000001", 1));

    seed_comic_cover(&mock, "comic--00000000001");
    seed_comic_cover(&mock, "comic--00000000002");

    delete(
        (&mock, &mock),
        token("user--00000000001"),
        "workset--00000000001".into(),
    )
    .await
    .unwrap();

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Comic)
            .await
            .unwrap()
    );

    let before_parent_sweep = mock.snapshot();

    assert!(before_parent_sweep.comics.is_empty());
    assert_eq!(before_parent_sweep.worksets.len(), 1);
    assert_eq!(before_parent_sweep.obj_tasks.len(), 2);

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Workset)
            .await
            .unwrap()
    );
    assert!(
        !sweep((&mock, &mock, &mock), SubtreeSweepLevel::Workset)
            .await
            .unwrap()
    );

    let snapshot = mock.snapshot();

    assert!(snapshot.worksets.is_empty());
    assert!(snapshot.deleted_workset_ids.is_empty());
    assert!(snapshot.deleted_comic_ids.is_empty());
}

#[tokio::test]
async fn delete_rolls_back_missing_workset() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001", 0));
    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));
    mock.seed_comic(comic("comic--00000000001", "workset--00000000001", 0));

    let cover_key = seed_comic_cover(&mock, "comic--00000000001");

    let error =
        delete((&mock, &mock), token("user--00000000001"), "missing".into())
            .await
            .err()
            .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Args);
    assert_eq!(snapshot.worksets.len(), 1);
    assert_eq!(snapshot.comics.len(), 1);
    assert_eq!(
        snapshot
            .objs
            .get("comic_cover")
            .unwrap()
            .get("comic--00000000001")
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .key,
        cover_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}
