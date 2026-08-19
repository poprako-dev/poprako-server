use super::super::reserve::validation::validate_page_count;

use crate::complex::image::ImageComplex;
use crate::part::prom::payload::image::ResourceKind;

#[test]
fn image_byte_length_accepts_closed_bounds() {
    //
    assert!(
        ImageComplex::ensure_byte_length(1, ResourceKind::PageImage).is_ok()
    );

    assert!(
        ImageComplex::ensure_byte_length(
            25 * 1024 * 1024,
            ResourceKind::PageImage,
        )
        .is_ok(),
    );
}

#[test]
fn image_byte_length_rejects_values_outside_bounds() {
    //
    assert!(
        ImageComplex::ensure_byte_length(0, ResourceKind::PageImage).is_err()
    );

    assert!(
        ImageComplex::ensure_byte_length(
            25 * 1024 * 1024 + 1,
            ResourceKind::PageImage,
        )
        .is_err(),
    );
}

#[test]
fn page_count_accepts_closed_bounds() {
    //
    assert!(validate_page_count(1).is_ok());

    assert!(validate_page_count(200).is_ok());
}

#[test]
fn page_count_rejects_values_outside_bounds() {
    //
    assert!(validate_page_count(0).is_err());

    assert!(validate_page_count(201).is_err());
}
