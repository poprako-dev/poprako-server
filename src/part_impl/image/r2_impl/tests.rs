// detect_content_type(detect_content_type)(positive): supported image extensions should map to MIME types.
// detect_content_type_rejects_unknown_extension(detect_content_type)(negative): unsupported extensions should be rejected.
// gen_download_url_uses_custom_domain(ImagePool::gen_download_url)(positive): download URLs should be built from the configured public domain.
// gen_thumbnail_download_url_uses_cloudflare_image_resizing(ImagePool::gen_thumbnail_download_url)(positive): thumbnail URLs should apply the configured Cloudflare Image Resizing options.

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
fn gen_download_url_uses_custom_domain() {
    //
    let url = build_public_url(
        "https://images.example.test/root/",
        "avatars/user-1.png",
        "gen_download_url",
    )
    .unwrap();

    assert_eq!(
        url.as_str(),
        "https://images.example.test/root/avatars/user-1.png"
    );
}

#[test]
fn gen_thumbnail_download_url_uses_cloudflare_image_resizing() {
    //
    let thumbnail_path = format!(
        "cdn-cgi/image/{}/{}",
        THUMBNAIL_TRANSFORM, "chapters/chapter-1/pages/page-1.jpg"
    );

    let url = build_public_url(
        "https://images.example.test/root/",
        &thumbnail_path,
        "gen_thumbnail_download_url",
    )
    .unwrap();

    assert_eq!(
        url.as_str(),
        "https://images.example.test/root/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/chapters/chapter-1/pages/page-1.jpg"
    );
}
