use super::AppConfig;

#[test]
fn parses_toml_http_settings() {
    //
    let config = AppConfig::parse(
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 524288
            team_avatar_limit = 524288
            comic_cover_limit = 2097152
            page_image_limit = 26214400
        "#,
    )
    .unwrap();

    assert_eq!(config.http.host, "127.0.0.1");

    assert_eq!(config.http.port, 8888);

    assert_eq!(config.image.user_avatar_limit, 524288);

    assert_eq!(config.image.team_avatar_limit, 524288);

    assert_eq!(config.image.comic_cover_limit, 2097152);

    assert_eq!(config.image.page_image_limit, 26214400);
}

#[test]
fn rejects_legacy_json_settings() {
    //
    let result = AppConfig::parse(
        r#"
            {
              "http": {
                "host": "127.0.0.1",
                "port": 8888
              }
            }
        "#,
    );

    assert!(result.is_err());
}

#[test]
fn rejects_zero_image_byte_limits() {
    //
    let zero_limit_configs = [
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 0
            team_avatar_limit = 1
            comic_cover_limit = 1
            page_image_limit = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 1
            team_avatar_limit = 0
            comic_cover_limit = 1
            page_image_limit = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 1
            team_avatar_limit = 1
            comic_cover_limit = 0
            page_image_limit = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 1
            team_avatar_limit = 1
            comic_cover_limit = 1
            page_image_limit = 0
        "#,
    ];

    for config_content in zero_limit_configs {
        //
        assert!(AppConfig::parse(config_content).is_err());
    }
}
