// detect_content_type(detect_content_type)(positive): supported image extensions should map to MIME types.
// detect_content_type_rejects_unknown_extension(detect_content_type)(negative): unsupported extensions should be rejected.
// gen_download_url_uses_custom_domain(ImagePool::gen_download_url)(positive): download URLs should be built from the configured public domain.

use super::*;

use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Region};

fn image_pool() -> R2ImagePool {
    //
    let credentials =
        Credentials::new("access-key", "secret-key", None, None, "test");

    let config = Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .endpoint_url("https://example.invalid")
        .credentials_provider(credentials)
        .build();

    R2ImagePool::new(
        Client::from_conf(config),
        "bucket".to_string(),
        "https://images.example.test/root/".to_string(),
    )
}

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

#[tokio::test]
async fn gen_download_url_uses_custom_domain() {
    //
    let image_pool = image_pool();

    let url =
        ImagePool::gen_download_url(&image_pool, "avatars/user-1.png").await;

    assert!(url.is_ok());

    let url = url.ok().unwrap();

    assert_eq!(
        url.as_str(),
        "https://images.example.test/root/avatars/user-1.png"
    );
}
