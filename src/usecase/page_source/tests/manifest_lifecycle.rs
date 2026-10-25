use super::super::alloc::alloc_chapter_page_sources;
use super::super::alloc::alloc_image;
use super::super::mark_image_uploaded;
use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::data::instr::page_source::{
    AllocChapterPageSourcesInstr, AllocPageSourceImageInstr,
    MarkPageSourceImageUploadedInstr, PageSourceImageInstr,
};
use crate::part_impl::prom::mock_impl::process_pending;
use crate::result::ExpectedVariant;
use crate::test_util::{IMAGE_CONFIG, assert_expected_variant};
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::role::RoleField;

const USER_ID: &str = "user--00000000001";
const CHAPTER_ID: &str = "chapter--00000000001";
const SOURCE_ID: &str = "page_source--00000000001";

fn seed_manifest_scope(mock: &Mock, page_source_count: usize) {
    seed_page_source_scope(mock, page_source_count);

    mock.seed_assignment(page_source_assignment(
        USER_ID,
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));
}

fn manifest_page(
    page_source_id: Option<&str>,
    hash: u8,
    new_byte_len: Option<u64>,
    ext: ImageExt,
) -> PageSourceImageInstr {
    PageSourceImageInstr {
        raw_ident: None,
        page_source_id: page_source_id.map(Into::into),
        image_hash: ImageHash::new([hash; 32]),
        new_byte_len,
        ext,
    }
}

async fn alloc_manifest(
    mock: &Mock,
    pages: Vec<PageSourceImageInstr>,
) -> crate::result::BaseRest<
    crate::data::val::page_source::AllocChapterPageSourcesVal,
> {
    alloc_chapter_page_sources(
        (mock, mock, mock, mock, &IMAGE_CONFIG),
        page_source_token(USER_ID),
        AllocChapterPageSourcesInstr {
            chapter_id: CHAPTER_ID.into(),
            pages,
        },
    )
    .await
}

#[tokio::test]
async fn retained_pending_image_without_byte_length_is_rejected() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_source_model(SOURCE_ID, 0));

    let existing_key =
        seed_page_source_obj(&mock, SOURCE_ID, 2, false, (0, ImageExt::Png));

    let error = alloc_manifest(
        &mock,
        vec![manifest_page(Some(SOURCE_ID), 0, None, ImageExt::Png)],
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Args);
    assert_eq!(snapshot.pages.first().unwrap().id, SOURCE_ID);
    assert_eq!(
        snapshot
            .objs
            .get("page_source_image")
            .unwrap()
            .get(SOURCE_ID)
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .key,
        existing_key
    );
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn retained_pending_image_with_byte_length_resigns_same_generation() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_source_model(SOURCE_ID, 0));

    let existing_key =
        seed_page_source_obj(&mock, SOURCE_ID, 2, false, (0, ImageExt::Png));

    let allocated = alloc_manifest(
        &mock,
        vec![manifest_page(Some(SOURCE_ID), 0, Some(4096), ImageExt::Png)],
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(
        allocated
            .pages
            .first()
            .unwrap()
            .slot
            .as_ref()
            .unwrap()
            .image_ver,
        2
    );
    assert_eq!(
        snapshot
            .objs
            .get("page_source_image")
            .unwrap()
            .get(SOURCE_ID)
            .unwrap()
            .version,
        2
    );
    assert_eq!(snapshot.obj_tasks.len(), 1);
    assert!(matches!(
        &snapshot.obj_tasks.first().unwrap().1,
        ObjTask::Check { key } if key == &existing_key
    ));
}

#[tokio::test]
async fn new_manifest_page_source_without_byte_length_is_rejected_before_writes()
 {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let error = alloc_manifest(
        &mock,
        vec![manifest_page(None, 1, None, ImageExt::Jpg)],
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Args);
    assert!(snapshot.pages.is_empty());
    assert!(snapshot.objs.is_empty());
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn changed_retained_image_without_byte_length_rolls_back_manifest() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_source_model(SOURCE_ID, 0));

    let existing_key =
        seed_page_source_obj(&mock, SOURCE_ID, 2, true, (0, ImageExt::Png));

    let error = alloc_manifest(
        &mock,
        vec![manifest_page(Some(SOURCE_ID), 1, None, ImageExt::Jpg)],
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Args);
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.pages.first().unwrap().id, SOURCE_ID);
    assert_eq!(snapshot.pages.first().unwrap().index, 0);
    assert_eq!(
        snapshot
            .objs
            .get("page_source_image")
            .unwrap()
            .get(SOURCE_ID)
            .unwrap()
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
async fn delayed_raw_advance_stays_pending_while_manifest_images_are_pending() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    alloc_manifest(
        &mock,
        vec![
            manifest_page(None, 1, Some(4096), ImageExt::Png),
            manifest_page(None, 2, Some(4096), ImageExt::Png),
        ],
    )
    .await
    .unwrap();

    process_pending(&mock).await.unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot
            .chapters
            .first()
            .unwrap()
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
    );
    assert!(
        snapshot
            .objs
            .get("page_source_image")
            .unwrap()
            .values()
            .all(|record| {
                record.meta.as_ref().is_some_and(|meta| !meta.is_avail)
            })
    );
}

#[tokio::test]
async fn invalid_manifest_count_is_rejected_without_side_effects() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let error = alloc_manifest(&mock, Vec::new()).await.err().unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(&error, ExpectedVariant::Args);
    assert!(snapshot.pages.is_empty());
    assert!(snapshot.objs.is_empty());
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn explicit_image_replacement_retains_page_source_and_deletes_old_generation()
 {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    let mut existing_page = page_source_model(SOURCE_ID, 0);

    existing_page.total_unit_count = 4;

    mock.seed_page(existing_page);

    let old_key =
        seed_page_source_obj(&mock, SOURCE_ID, 7, true, (0, ImageExt::Png));

    let allocated = alloc_manifest(
        &mock,
        vec![manifest_page(Some(SOURCE_ID), 1, Some(8192), ImageExt::Jpg)],
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(allocated.pages.first().unwrap().page_source_id, SOURCE_ID);
    assert_eq!(
        allocated
            .pages
            .first()
            .unwrap()
            .slot
            .as_ref()
            .unwrap()
            .image_ver,
        8
    );
    assert_eq!(snapshot.pages.first().unwrap().total_unit_count, 4);
    assert_eq!(snapshot.chapters.first().unwrap().total_unit_count, 4);
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key == &old_key)
    }));
}

#[tokio::test]
async fn delayed_raw_advance_completes_after_every_generation_is_marked() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let allocated = alloc_manifest(
        &mock,
        vec![
            manifest_page(None, 1, Some(4096), ImageExt::Png),
            manifest_page(None, 2, Some(4096), ImageExt::Png),
        ],
    )
    .await
    .unwrap();

    for page in allocated.pages {
        let image_ver = page.slot.unwrap().image_ver;

        mark_image_uploaded(
            (&mock, &mock, &mock),
            page_source_token(USER_ID),
            page.page_source_id,
            MarkPageSourceImageUploadedInstr { image_ver },
        )
        .await
        .unwrap();
    }

    process_pending(&mock).await.unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot
            .chapters
            .first()
            .unwrap()
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Completed)
    );
    assert_eq!(snapshot.chapter_workflow_records.len(), 1);
}

#[tokio::test]
async fn published_chapter_rejects_allocations_and_confirmation() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 1);

    mock.state
        .lock()
        .unwrap()
        .chapters
        .get_mut(0)
        .unwrap()
        .stages = StageMask::try_from(0u32)
        .unwrap()
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_page(page_source_model(SOURCE_ID, 0));

    seed_page_source_obj(&mock, SOURCE_ID, 1, false, (0, ImageExt::Png));

    let manifest_result = alloc_manifest(
        &mock,
        vec![manifest_page(Some(SOURCE_ID), 0, Some(4096), ImageExt::Png)],
    )
    .await;

    assert!(manifest_result.is_err());

    let image_result = alloc_image(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        page_source_token(USER_ID),
        SOURCE_ID.into(),
        AllocPageSourceImageInstr {
            raw_ident: None,
            image_hash: ImageHash::new([1; 32]),
            new_byte_len: 4096,
            ext: ImageExt::Jpg,
        },
    )
    .await;

    assert!(image_result.is_err());

    let result = mark_image_uploaded(
        (&mock, &mock, &mock),
        page_source_token(USER_ID),
        SOURCE_ID.into(),
        MarkPageSourceImageUploadedInstr { image_ver: 1 },
    )
    .await;

    assert!(result.is_err());

    let snapshot = mock.snapshot();

    assert!(
        !snapshot
            .objs
            .get("page_source_image")
            .unwrap()
            .get(SOURCE_ID)
            .unwrap()
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn raw_ident_is_replaced_after_manifest_matching_and_reordering() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    let mut first = manifest_page(None, 1, Some(4096), ImageExt::Png);

    first.raw_ident = Some("first.JPG".into());

    let mut second = manifest_page(None, 2, Some(4096), ImageExt::Png);

    second.raw_ident = Some("second.PNG".into());

    let allocated = alloc_manifest(&mock, vec![first, second]).await.unwrap();

    let first_id = allocated.pages.first().unwrap().page_source_id.clone();

    let second_id = allocated.pages.get(1).unwrap().page_source_id.clone();

    for page in &allocated.pages {
        mark_image_uploaded(
            (&mock, &mock, &mock),
            page_source_token(USER_ID),
            page.page_source_id.clone(),
            MarkPageSourceImageUploadedInstr {
                image_ver: page.image_ver,
            },
        )
        .await
        .unwrap();
    }

    let mut second = manifest_page(Some(&second_id), 2, None, ImageExt::Png);

    second.raw_ident = Some("renamed.PNG".into());

    let reordered = alloc_manifest(
        &mock,
        vec![
            second,
            manifest_page(Some(&first_id), 1, None, ImageExt::Png),
        ],
    )
    .await
    .unwrap();

    assert_eq!(reordered.pages.first().unwrap().page_source_id, second_id);
    assert_eq!(reordered.pages.get(1).unwrap().page_source_id, first_id);
    assert!(reordered.pages.iter().all(|page| page.slot.is_none()));

    let snapshot = mock.snapshot();

    assert!(!snapshot.page_source_raw_idents.contains_key(&first_id));
    assert_eq!(
        snapshot
            .page_source_raw_idents
            .get(&second_id)
            .unwrap()
            .raw_ident,
        "renamed.PNG"
    );

    let mut retained = manifest_page(Some(&second_id), 2, None, ImageExt::Png);

    retained.raw_ident = None;

    alloc_manifest(&mock, vec![retained]).await.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);
    assert!(snapshot.page_source_raw_idents.is_empty());
}

#[tokio::test]
async fn manifest_failure_rolls_back_raw_ident_assignment() {
    let mock = Mock::new().with_obj_delete_failure();

    seed_manifest_scope(&mock, 1);

    mock.seed_page(page_source_model(SOURCE_ID, 0));

    seed_page_source_obj(&mock, SOURCE_ID, 1, true, (0, ImageExt::Png));

    let mut replacement = manifest_page(None, 2, Some(4096), ImageExt::Png);

    replacement.raw_ident = Some("replacement.png".into());

    assert!(alloc_manifest(&mock, vec![replacement]).await.is_err());

    let snapshot = mock.snapshot();

    assert!(snapshot.page_source_raw_idents.is_empty());
    assert_eq!(snapshot.pages.len(), 1);
    assert_eq!(snapshot.pages.first().unwrap().id, SOURCE_ID);
    assert_eq!(
        snapshot
            .objs
            .get("page_source_image")
            .unwrap()
            .get(SOURCE_ID)
            .unwrap()
            .version,
        1
    );
}

#[tokio::test]
async fn repeated_reallocation_does_not_retain_obsolete_raw_idents() {
    let mock = Mock::new();

    seed_manifest_scope(&mock, 0);

    for hash in 1..=16 {
        let mut page = manifest_page(None, hash, Some(4096), ImageExt::Png);

        page.raw_ident = Some(format!("source-{hash}.png"));

        let allocated = alloc_manifest(&mock, vec![page]).await.unwrap();

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.pages.len(), 1);
        assert_eq!(snapshot.page_source_raw_idents.len(), 1);
        assert!(
            snapshot
                .page_source_raw_idents
                .contains_key(&allocated.pages.first().unwrap().page_source_id)
        );
    }

    assert!(alloc_manifest(&mock, vec![]).await.is_err());

    assert_eq!(mock.snapshot().page_source_raw_idents.len(), 1);

    alloc_manifest(
        &mock,
        vec![manifest_page(None, 16, Some(4096), ImageExt::Png)],
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);
    assert!(snapshot.page_source_raw_idents.is_empty());
}
