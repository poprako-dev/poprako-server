use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::data::instr::user::AllocUserAvatarInstr;
use crate::test_util::IMAGE_CONFIG;
use crate::value::image::{ImageExt, ImageHash};

fn alloc_instr(hash_byte: u8, ext: ImageExt) -> AllocUserAvatarInstr {
    AllocUserAvatarInstr {
        image_hash: ImageHash::new([hash_byte; 32]),
        new_byte_len: 4096,
        ext,
    }
}

fn seed_user_scope(mock: &Mock) {
    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );
}

#[tokio::test]
async fn alloc_avatar_creates_generation_check_and_put_url() {
    let mock = Mock::new();

    seed_user_scope(&mock);

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .unwrap();

    let slot = allocated.slot.unwrap();

    assert_eq!(slot.image_ver, 1);
    assert_eq!(
        slot.put_url,
        "https://obj.test/write/user_avatar/user-1-1.png",
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.objs["user_avatar"]["user-1"].version, 1);
    assert!(matches!(snapshot.obj_tasks[0].1, ObjTask::Check { .. }));
}

#[tokio::test]
async fn alloc_avatar_replacement_deletes_old_and_checks_new_generation() {
    let mock = Mock::new();

    seed_user_scope(&mock);
    seed_user_avatar(&mock, "user-1", 1);

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        alloc_instr(2, ImageExt::Jpg),
    )
    .await
    .unwrap();

    assert_eq!(allocated.slot.unwrap().image_ver, 2);

    let snapshot = mock.snapshot();

    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key.ver == 1)
    }));
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Check { key, .. } if key.ver == 2)
    }));
}

#[tokio::test]
async fn alloc_avatar_rolls_back_when_user_is_missing() {
    let mock = Mock::new();

    let err = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().objs.is_empty());
    assert!(mock.snapshot().obj_tasks.is_empty());
}

#[tokio::test]
async fn alloc_avatar_returns_no_slot_for_same_available_content() {
    let mock = Mock::new();

    seed_user_scope(&mock);
    seed_user_avatar(&mock, "user-1", 1);

    mock.state
        .lock()
        .unwrap()
        .objs
        .get_mut("user_avatar")
        .unwrap()
        .get_mut("user-1")
        .unwrap()
        .meta
        .as_mut()
        .unwrap()
        .is_avail = true;

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        alloc_instr(0, ImageExt::Png),
    )
    .await
    .unwrap();

    assert!(allocated.slot.is_none());
    assert!(mock.snapshot().obj_tasks.is_empty());
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_old_allocation_replay() {
    let mock = Mock::new();

    seed_user_scope(&mock);
    seed_user_avatar(&mock, "user-1", 1);

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .unwrap();

    assert_eq!(allocated.slot.unwrap().image_ver, 2);

    let err = mark_avatar_uploaded::<MockContext, _>(
        (&mock,),
        token("user-1"),
        "user-1".into(),
        MarkUserAvatarUploadedInstr { image_ver: 1 },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(mock.snapshot().objs["user_avatar"]["user-1"].version, 2);
}

#[tokio::test]
async fn mark_avatar_uploaded_accepts_repeated_current_generation() {
    let mock = Mock::new();

    seed_user_avatar(&mock, "user-1", 3);

    for _ in 0..2 {
        mark_avatar_uploaded::<MockContext, _>(
            (&mock,),
            token("user-1"),
            "user-1".into(),
            MarkUserAvatarUploadedInstr { image_ver: 3 },
        )
        .await
        .unwrap();
    }

    assert!(
        mock.snapshot().objs["user_avatar"]["user-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}
