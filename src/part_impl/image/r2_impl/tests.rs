// detect_content_type(detect_content_type)(positive): supported image extensions should map to MIME types.
// detect_content_type_rejects_unknown_extension(detect_content_type)(negative): unsupported extensions should be rejected.
// get_signed_uses_custom_domain(ImagePool::get_signed)(positive): download URLs should be built from the configured public domain.

use super::*;

#[test]
fn detect_content_type_maps_supported_extensions() {
    //
    assert_eq!(detect_content_type("avatar.PNG"), Some("image/png"));

    assert_eq!(detect_content_type("avatar.webp"), Some("image/webp"));
}

#[test]
fn detect_content_type_rejects_unknown_extension() {
    assert_eq!(detect_content_type("avatar.txt"), None);
}

#[test]
fn get_signed_uses_custom_domain() {
    //
    let url = R2ImagePool::public_url(
        "https://images.example.test/root/",
        "avatars/user-1.png",
    );

    assert!(url.is_ok());

    let url = url.ok().unwrap();

    assert_eq!(
        url.as_str(),
        "https://images.example.test/root/avatars/user-1.png"
    );
}
