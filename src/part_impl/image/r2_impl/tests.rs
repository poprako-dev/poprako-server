// gen_download_url_uses_custom_domain(ImagePool::gen_download_url)(positive): download URLs should be built from the configured public domain.
// gen_thumbnail_download_url_uses_cloudflare_image_resizing(ImagePool::gen_thumbnail_download_url)(positive): thumbnail URLs should apply the configured Cloudflare Image Resizing options.

use super::*;

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
