use super::*;

use time::OffsetDateTime;

use crate::value::image::{ImageExt, ImageHash};

fn page(
    id: &str,
    index: i32,
    hash: u8,
    total_unit_count: i32,
    image_uploaded: bool,
) -> PageInfo {
    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        image_key: Some(format!("{}.png", id)),
        image_uploaded,
        image_version: 1,
        image_hash: ImageHash::new([hash; 32]),
        image_byte_length: 4096,
        image_ext: ImageExt::Png,
        total_unit_count,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn input(page_id: Option<&str>, hash: u8) -> PageImageSpec {
    PageImageSpec {
        page_id: page_id.map(Into::into),
        image_hash: ImageHash::new([hash; 32]),
        byte_length: 4096,
        ext: ImageExt::Png,
    }
}

#[test]
fn explicit_identity_is_reserved_before_automatic_matching() {
    //
    let existing_page_infos = vec![
        page("page-a", 0, 0, 0, true),
        page("page-b", 1, 0, 10, true),
    ];

    let page_inputs = vec![input(None, 0), input(Some("page-b"), 0)];

    let manifest_plan =
        build("chapter-1", &existing_page_infos, &page_inputs).unwrap();

    assert_eq!(manifest_plan.matches[0].existing_index, Some(0));

    assert_eq!(manifest_plan.matches[1].existing_index, Some(1));
}

#[test]
fn automatic_matching_uses_units_uploaded_index_and_id_priority() {
    //
    let existing_page_infos = vec![
        page("page-z", 0, 0, 0, true),
        page("page-b", 3, 0, 1, false),
        page("page-c", 2, 0, 1, true),
        page("page-a", 2, 0, 1, true),
    ];

    let page_inputs = vec![input(None, 0), input(None, 0), input(None, 0)];

    let manifest_plan =
        build("chapter-1", &existing_page_infos, &page_inputs).unwrap();

    assert_eq!(manifest_plan.matches[0].existing_index, Some(3));

    assert_eq!(manifest_plan.matches[1].existing_index, Some(2));

    assert_eq!(manifest_plan.matches[2].existing_index, Some(1));

    assert_eq!(manifest_plan.deleted_existing_indexes, vec![0]);
}

#[test]
fn explicit_identity_can_replace_the_image_hash() {
    //
    let existing_page_infos = vec![page("page-a", 0, 0, 4, true)];

    let page_inputs = vec![input(Some("page-a"), 1)];

    let manifest_plan =
        build("chapter-1", &existing_page_infos, &page_inputs).unwrap();

    assert_eq!(manifest_plan.matches[0].existing_index, Some(0));

    assert!(manifest_plan.deleted_existing_indexes.is_empty());
}

#[test]
fn same_hash_with_conflicting_metadata_is_rejected() {
    //
    let existing_page_infos = vec![page("page-a", 0, 0, 0, true)];

    let mut page_input = input(None, 0);

    page_input.byte_length = 4097;

    let result = build("chapter-1", &existing_page_infos, &[page_input]);

    assert!(matches!(result, Err(BaseError::Expected { .. })));
}
