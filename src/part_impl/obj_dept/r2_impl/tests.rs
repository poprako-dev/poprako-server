// gen_url_uses_custom_domain(ObjPool::gen_url)(positive): download URLs should be built from the configured public domain.
// gen_thumbnail_download_url_uses_cloudflare_image_resizing(ObjPool::gen_get_url)(positive): thumbnail URLs should apply the configured Cloudflare Image Resizing options.

use super::*;

#[test]
fn gen_download_url_uses_custom_domain() {
    //
    let url = build_public_url(
        "https://images.example.test/root/",
        "avatars/user-1.png",
    )
    .unwrap();

    assert_eq!(
        url.as_str(),
        "https://images.example.test/root/avatars/user-1.png"
    );
}
