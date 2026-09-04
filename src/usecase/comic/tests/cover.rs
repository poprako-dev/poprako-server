use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::data::instr::comic::AllocComicCoverInstr;
use crate::test_util::IMAGE_CONFIG;
use crate::test_util::fixture::workset;
use crate::usecase::subtree_delete::sweep;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::subtree_delete::SubtreeSweepLevel;

fn alloc_instr(hash_byte: u8) -> AllocComicCoverInstr {
    AllocComicCoverInstr {
        image_hash: ImageHash::new([hash_byte; 32]),
        new_byte_len: 4096,
        ext: ImageExt::Png,
    }
}

fn seed_alloc_scope(mock: &Mock) {
    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1", 0));
}

#[tokio::test]
async fn alloc_cover_updates_object_state_enqueues_check_and_returns_put_url() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let allocated = alloc::alloc_cover::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "comic-1".into(),
        alloc_instr(1),
    )
    .await
    .unwrap();

    let slot = allocated.slot.unwrap();

    assert_eq!(slot.image_ver, 1);
    assert_eq!(
        slot.put_url,
        "https://obj.test/write/comic_cover/comic-1-1.png",
    );

    let snapshot = mock.snapshot();

    let record = &snapshot.objs["comic_cover"]["comic-1"];

    assert_eq!(record.version, 1);
    assert!(!record.meta.as_ref().unwrap().is_avail);
    assert_eq!(snapshot.obj_tasks.len(), 1);
    assert!(matches!(snapshot.obj_tasks[0].1, ObjTask::Check { .. }));
}

#[tokio::test]
async fn alloc_cover_rolls_back_missing_comic() {
    let mock = Mock::new();

    let err = alloc::alloc_cover::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "missing".into(),
        alloc_instr(1),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().objs.is_empty());
    assert!(mock.snapshot().obj_tasks.is_empty());
}

#[tokio::test]
async fn mark_cover_uploaded_marks_matching_generation() {
    let mock = Mock::new();

    seed_comic_cover_scope(&mock, 2);

    mark_comic_cover(&mock, 2).await.unwrap();

    assert!(
        mock.snapshot().objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn mark_cover_uploaded_accepts_repeated_matching_generation() {
    let mock = Mock::new();

    seed_comic_cover_scope(&mock, 2);

    mark_comic_cover(&mock, 2).await.unwrap();
    mark_comic_cover(&mock, 2).await.unwrap();

    assert!(
        mock.snapshot().objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn mark_cover_uploaded_rejects_stale_generation() {
    let mock = Mock::new();

    seed_comic_cover_scope(&mock, 2);

    let err = mark_comic_cover(&mock, 1).await.err().unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(
        !mock.snapshot().objs["comic_cover"]["comic-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn mark_cover_uploaded_rejects_old_allocation_replay() {
    let mock = Mock::new();

    seed_comic_cover_scope(&mock, 1);

    let allocated = alloc::alloc_cover::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "comic-1".into(),
        alloc_instr(1),
    )
    .await
    .unwrap();

    assert_eq!(allocated.slot.unwrap().image_ver, 2);

    let err = mark_comic_cover(&mock, 1).await.err().unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let record = &mock.snapshot().objs["comic_cover"]["comic-1"];

    assert_eq!(record.version, 2);
    assert!(!record.meta.as_ref().unwrap().is_avail);
}

#[tokio::test]
async fn delete_marks_comic_then_sweep_removes_cover() {
    let mock = Mock::new();

    seed_comic_cover_scope(&mock, 1);

    mock.state.lock().unwrap().worksets[0].comic_count = 1;

    delete::<_, MockContext, _>(
        (&mock, &mock),
        token("user-1"),
        "comic-1".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.len(), 1);
    assert!(snapshot.deleted_comic_ids.contains("comic-1"));
    assert_eq!(snapshot.worksets[0].comic_count, 0);
    assert!(snapshot.obj_tasks.is_empty());

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Comic,)
            .await
            .unwrap()
    );

    let swept_snapshot = mock.snapshot();

    assert!(swept_snapshot.comics.is_empty());
    assert!(swept_snapshot.objs["comic_cover"].is_empty());
    assert!(swept_snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key.id == "comic-1")
    }));
}

#[tokio::test]
async fn delete_rolls_back_missing_comic() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_member(admin_member("user-1", "team-1"));

    let err = delete::<_, MockContext, _>(
        (&mock, &mock),
        token("user-1"),
        "missing".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(mock.snapshot().worksets.len(), 1);
    assert!(mock.snapshot().obj_tasks.is_empty());
}
