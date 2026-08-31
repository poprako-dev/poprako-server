use super::super::alloc::alloc_image;
use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::data::instr::page::AllocPageImageInstr;
use crate::result::ExpectedVariant;
use crate::test_util::{IMAGE_CONFIG, assert_expected_variant};
use crate::value::role::RoleField;

fn seed_alloc_scope(mock: &Mock) {
    seed_page_scope(mock, 1);

    mock.seed_page(page_model("page-1", 0));

    mock.seed_assignment(page_assignment(
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));
}

fn alloc_instr(hash: u8, ext: ImageExt) -> AllocPageImageInstr {
    AllocPageImageInstr {
        image_hash: ImageHash::new([hash; 32]),
        new_byte_len: 4096,
        ext,
    }
}

#[tokio::test]
async fn first_image_allocation_creates_generation_and_check_task() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let allocated = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_token("user-1"),
        "page-1".into(),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .unwrap();

    let slot = allocated.slot.unwrap();

    assert_eq!(slot.image_ver, 1);
    assert_eq!(
        slot.put_url,
        "https://obj.test/write/page/chapter_chapter-1/page-1-1.png"
    );

    let snapshot = mock.snapshot();
    let record = &snapshot.objs["page_image"]["page-1"];

    assert_eq!(record.version, 1);
    assert!(!record.meta.as_ref().unwrap().is_avail);
    assert_eq!(snapshot.obj_tasks.len(), 1);
    assert!(matches!(snapshot.obj_tasks[0].1, ObjTask::Check { .. }));
    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn replacement_allocation_deletes_old_generation_and_checks_new_one() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let old_key = seed_page_obj(&mock, "page-1", 4, true, 0, ImageExt::Png);

    let allocated = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_token("user-1"),
        "page-1".into(),
        alloc_instr(1, ImageExt::Jpg),
    )
    .await
    .unwrap();

    let slot = allocated.slot.unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(slot.image_ver, 5);
    assert_eq!(snapshot.objs["page_image"]["page-1"].version, 5);
    assert_eq!(snapshot.obj_tasks.len(), 2);
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key == &old_key)
    }));
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Check { key } if key.ver == 5)
    }));
}

#[tokio::test]
async fn matching_available_image_returns_no_slot_without_version_bump() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let existing_key =
        seed_page_obj(&mock, "page-1", 4, true, 0, ImageExt::Png);

    let allocated = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_token("user-1"),
        "page-1".into(),
        alloc_instr(0, ImageExt::Png),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(allocated.slot.is_none());
    assert_eq!(snapshot.objs["page_image"]["page-1"].version, 4);
    assert_eq!(
        snapshot.objs["page_image"]["page-1"]
            .meta
            .as_ref()
            .unwrap()
            .key,
        existing_key
    );
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn matching_pending_image_resigns_current_generation() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let existing_key =
        seed_page_obj(&mock, "page-1", 4, false, 0, ImageExt::Png);

    let allocated = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_token("user-1"),
        "page-1".into(),
        alloc_instr(0, ImageExt::Png),
    )
    .await
    .unwrap();

    let slot = allocated.slot.unwrap();
    let snapshot = mock.snapshot();

    assert_eq!(slot.image_ver, 4);
    assert!(slot.put_url.ends_with(&existing_key.image));
    assert_eq!(snapshot.objs["page_image"]["page-1"].version, 4);
    assert_eq!(snapshot.obj_tasks.len(), 1);
    assert!(matches!(
        &snapshot.obj_tasks[0].1,
        ObjTask::Check { key } if key == &existing_key
    ));
}

#[tokio::test]
async fn matching_hash_with_different_extension_allocates_new_generation() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let old_key = seed_page_obj(&mock, "page-1", 4, true, 0, ImageExt::Png);

    let allocated = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_token("user-1"),
        "page-1".into(),
        alloc_instr(0, ImageExt::Webp),
    )
    .await
    .unwrap();

    let slot = allocated.slot.unwrap();
    let snapshot = mock.snapshot();
    let current_meta =
        snapshot.objs["page_image"]["page-1"].meta.as_ref().unwrap();

    assert_eq!(slot.image_ver, 5);
    assert_eq!(current_meta.ext, "webp");
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key == &old_key)
    }));
}

#[tokio::test]
async fn missing_page_rejects_image_allocation_without_side_effects() {
    let mock = Mock::new();

    let error = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_token("user-1"),
        "missing".into(),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(error, ExpectedVariant::Args);
    assert!(snapshot.objs.is_empty());
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}
