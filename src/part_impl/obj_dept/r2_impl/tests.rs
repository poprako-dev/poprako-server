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

#[tokio::test]
async fn gen_urls_includes_cloudflare_thumbnail_transform() {
    let config = Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .credentials_provider(Credentials::new("a", "b", None, None, "test"))
        .build();

    let pool = R2ObjPool::new(
        Client::from_conf(config),
        String::from("bucket"),
        String::from("https://images.example.test"),
    );

    let urls = pool
        .gen_urls("page_image/page-1/1.png", ObjUrlProfile::ImageThumbnail)
        .await
        .unwrap();

    assert_eq!(
        urls.origin_url.as_str(),
        "https://images.example.test/page_image/page-1/1.png",
    );

    assert_eq!(
        urls.thumbnail_url.as_ref().map(Url::as_str),
        Some(
            "https://images.example.test/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/page_image/page-1/1.png",
        ),
    );
}

#[tokio::test]
async fn origin_only_profile_omits_image_thumbnail() {
    let config = Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .credentials_provider(Credentials::new("a", "b", None, None, "test"))
        .build();

    let pool = R2ObjPool::new(
        Client::from_conf(config),
        String::from("bucket"),
        String::from("https://objects.example.test"),
    );

    let urls = pool
        .gen_urls("font_file/font-1/1", ObjUrlProfile::OriginOnly)
        .await
        .unwrap();

    assert_eq!(
        urls.origin_url.as_str(),
        "https://objects.example.test/font_file/font-1/1",
    );

    assert!(urls.thumbnail_url.is_none());
}
