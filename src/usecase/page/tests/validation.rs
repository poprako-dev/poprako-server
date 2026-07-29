use super::super::reserve::{validate_image_byte_length, validate_page_count};

#[test]
fn image_byte_length_accepts_closed_bounds() {
    //
    assert!(validate_image_byte_length(1).is_ok());

    assert!(validate_image_byte_length(20 * 1024 * 1024).is_ok());
}

#[test]
fn image_byte_length_rejects_values_outside_bounds() {
    //
    assert!(validate_image_byte_length(0).is_err());

    assert!(validate_image_byte_length(20 * 1024 * 1024 + 1).is_err());
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
