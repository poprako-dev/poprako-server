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

#[test]
fn gen_urls_includes_selected_cloudflare_image_transforms() {
    let obj_url_spec = ObjUrlSpec::default()
        .with_origin()
        .with_optimized()
        .with_thumbnail();

    let urls = build_obj_urls(
        "https://images.example.test",
        "page_image/page-1/1.png",
        obj_url_spec,
    )
    .unwrap();

    assert_eq!(
        urls.origin_url.as_ref().map(Url::as_str),
        Some("https://images.example.test/page_image/page-1/1.png"),
    );

    assert_eq!(
        urls.optimized_url.as_ref().map(Url::as_str),
        Some(
            "https://images.example.test/cdn-cgi/image/width=1080,fit=scale-down,quality=80,format=auto,metadata=none/page_image/page-1/1.png",
        ),
    );

    assert_eq!(
        urls.thumbnail_url.as_ref().map(Url::as_str),
        Some(
            "https://images.example.test/cdn-cgi/image/width=300,fit=scale-down,quality=80,format=auto,metadata=none/page_image/page-1/1.png",
        ),
    );
}

#[test]
fn origin_only_spec_omits_image_renditions() {
    let obj_url_spec = ObjUrlSpec::default().with_origin();

    let urls = build_obj_urls(
        "https://objects.example.test",
        "font_file/font-1/1",
        obj_url_spec,
    )
    .unwrap();

    assert_eq!(
        urls.origin_url.as_ref().map(Url::as_str),
        Some("https://objects.example.test/font_file/font-1/1"),
    );

    assert!(urls.optimized_url.is_none());

    assert!(urls.thumbnail_url.is_none());
}

#[test]
fn empty_url_spec_is_invalid() {
    let error = build_obj_urls(
        "https://objects.example.test",
        "font_file/font-1/1",
        ObjUrlSpec::default(),
    )
    .unwrap_err();

    assert!(matches!(error, ObjDeptError::Invalid { .. }));
}
