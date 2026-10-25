#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;
use crate::complex::page_source::manifest as page_source_manifest_complex;

use crate::value::image::{ImageExt, ImageHash};

fn candidate(id: &'static str) -> PageSourceManifestCand<'static> {
    PageSourceManifestCand {
        id,
        chapter_id: "chapter--00000000001",
    }
}

fn input(
    page_source_id: Option<&str>,
    hash: u8,
    ext: ImageExt,
) -> PageSourceImageSpec {
    PageSourceImageSpec {
        raw_ident: None,
        page_source_id: page_source_id.map(Into::into),
        image_hash: ImageHash::new([hash; 32]),
        new_byte_len: Some(4096),
        ext,
    }
}

#[test]
fn absent_identity_creates_a_new_page_even_beside_explicit_identity() {
    let candidates = vec![
        candidate("page_source--34844962250"),
        candidate("page_source--35727087946"),
    ];
    let inputs = vec![
        input(None, 0, ImageExt::Png),
        input(Some("page_source--35727087946"), 0, ImageExt::Png),
    ];

    let plan = page_source_manifest_complex::build(
        "chapter--00000000001",
        &candidates,
        &inputs,
    )
    .unwrap();

    assert_eq!(plan.matches.first().unwrap().existing_index, None);
    assert_eq!(plan.matches.get(1).unwrap().existing_index, Some(1));
}

#[test]
fn absent_identities_delete_all_omitted_pages() {
    let candidates = vec![
        candidate("page_source--43261075755"),
        candidate("page_source--35727087946"),
        candidate("page_source--92070664930"),
        candidate("page_source--34844962250"),
    ];
    let inputs = vec![
        input(None, 0, ImageExt::Png),
        input(None, 0, ImageExt::Png),
        input(None, 0, ImageExt::Png),
    ];

    let plan = page_source_manifest_complex::build(
        "chapter--00000000001",
        &candidates,
        &inputs,
    )
    .unwrap();

    assert_eq!(plan.matches.first().unwrap().existing_index, None);
    assert_eq!(plan.matches.get(1).unwrap().existing_index, None);
    assert_eq!(plan.matches.get(2).unwrap().existing_index, None);
    assert_eq!(plan.deleted_existing_indexes, vec![0, 1, 2, 3]);
}

#[test]
fn explicit_identity_can_replace_the_image_hash() {
    let candidates = vec![candidate("page_source--34844962250")];
    let inputs =
        vec![input(Some("page_source--34844962250"), 1, ImageExt::Png)];

    let plan = page_source_manifest_complex::build(
        "chapter--00000000001",
        &candidates,
        &inputs,
    )
    .unwrap();

    assert_eq!(plan.matches.first().unwrap().existing_index, Some(0));
    assert!(plan.deleted_existing_indexes.is_empty());
}

#[test]
fn same_hash_with_different_extension_is_a_distinct_identity() {
    let candidates = vec![candidate("page_source--34844962250")];
    let inputs = vec![input(None, 0, ImageExt::Jpg)];

    let plan = page_source_manifest_complex::build(
        "chapter--00000000001",
        &candidates,
        &inputs,
    )
    .unwrap();

    assert!(plan.matches.first().unwrap().existing_index.is_none());
    assert_eq!(plan.deleted_existing_indexes, vec![0]);
}
