use super::super::alloc::validation::validate_page_source_count;

use crate::complex::image as image_complex;
use crate::complex::page_source as page_source_complex;
use crate::data::instr::page_source::{
    AllocPageSourceImageInstr, PageSourceImageInstr,
};
use crate::test_util::IMAGE_CONFIG;
use crate::value::image::ImageHash;
use crate::value::image::ImageKind;
use serde_json::json;

#[test]
fn image_byte_length_accepts_closed_bounds() {
    //
    assert!(
        image_complex::ensure_byte_length(
            &IMAGE_CONFIG,
            1,
            ImageKind::PageSourceImage,
        )
        .is_ok(),
    );

    assert!(
        image_complex::ensure_byte_length(
            &IMAGE_CONFIG,
            25 * 1024 * 1024,
            ImageKind::PageSourceImage,
        )
        .is_ok(),
    );
}

#[test]
fn image_byte_length_rejects_values_outside_bounds() {
    //
    assert!(
        image_complex::ensure_byte_length(
            &IMAGE_CONFIG,
            0,
            ImageKind::PageSourceImage,
        )
        .is_err(),
    );

    assert!(
        image_complex::ensure_byte_length(
            &IMAGE_CONFIG,
            25 * 1024 * 1024 + 1,
            ImageKind::PageSourceImage,
        )
        .is_err(),
    );
}

#[test]
fn page_source_count_accepts_closed_bounds() {
    //
    assert!(validate_page_source_count(1).is_ok());

    assert!(validate_page_source_count(200).is_ok());
}

#[test]
fn page_source_count_rejects_values_outside_bounds() {
    //
    assert!(validate_page_source_count(0).is_err());

    assert!(validate_page_source_count(201).is_err());
}

#[test]
fn raw_ident_deserializes_optional_filenames_for_both_allocations() {
    for (input, expected) in [
        (None, None),
        (Some(json!(null)), None),
        (Some(json!("原稿 01.PNG")), Some("原稿 01.PNG".to_owned())),
    ] {
        let mut body = json!({
            "image_hash": ImageHash::new([1; 32]),
            "new_byte_len": 4096,
            "ext": "png",
        });

        if let Some(input) = input {
            body.as_object_mut()
                .unwrap()
                .insert("raw_ident".into(), input);
        }

        let single: AllocPageSourceImageInstr =
            serde_json::from_value(body.clone()).unwrap();

        let manifest: PageSourceImageInstr =
            serde_json::from_value(body).unwrap();

        assert_eq!(single.raw_ident, expected);
        assert_eq!(manifest.raw_ident, expected);
    }
}

#[test]
fn raw_ident_rejects_patch_objects_and_invalid_filenames() {
    for raw_ident in [
        json!({"type": "skip"}),
        json!({"type": "clear"}),
        json!({"type": "assign", "value": "name.png"}),
        json!(42),
    ] {
        let body = json!({
            "image_hash": ImageHash::new([1; 32]),
            "new_byte_len": 4096,
            "ext": "png",
            "raw_ident": raw_ident,
        });

        assert!(
            serde_json::from_value::<AllocPageSourceImageInstr>(body.clone())
                .is_err()
        );
        assert!(serde_json::from_value::<PageSourceImageInstr>(body).is_err());
    }

    for value in [
        "", "  ", "a\nb.png", "a\rb.png", "a\0b.png", "a/b.png", "a\\b.png",
        "a\tb.png",
    ] {
        assert!(page_source_complex::ensure_raw_ident(Some(value)).is_err());
    }

    for value in ["原稿 01.PNG", " name.webp ", "[封面].png", "same.jpg"] {
        assert!(page_source_complex::ensure_raw_ident(Some(value)).is_ok());
    }

    assert!(page_source_complex::ensure_raw_ident(None).is_ok());
}
