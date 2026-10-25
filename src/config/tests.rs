#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::AppConfig;
use std::time::Duration;

// Check every default, including sections omitted from a partial TOML file.
fn assert_defaults(config: &AppConfig) {
    assert_eq!(config.http.host, "127.0.0.1");

    assert_eq!(config.http.port, 8888);

    assert_eq!(config.http.cors_origin.header(), "https://poprako.com");

    assert_eq!(config.http.rate_limit.requests_per_second.get(), 20);

    assert_eq!(config.http.rate_limit.burst.get(), 80);

    assert_eq!(config.image.user_avatar_limit_mib, 1);

    assert_eq!(config.image.team_avatar_limit_mib, 1);

    assert_eq!(config.image.comic_cover_limit_mib, 2);

    assert_eq!(config.image.page_source_image_limit_mib, 25);

    assert_eq!(config.image.page_artwork_image_limit_mib, 25);

    assert_eq!(config.artwork.chapter_artwork_limit_mib, 512);

    assert_eq!(config.cache.unit_save_limit_mib.get(), 256);

    assert_eq!(config.cache.unit_save_token_budget.get(), 500_000);

    assert_eq!(config.cache.unit_save_ttl, Duration::from_hours(30 * 24));

    assert_eq!(config.cache.online_user_ttl, Duration::from_mins(10));

    assert_eq!(config.repo.subtree_sweep_batch_size.get(), 64);

    assert_eq!(config.repo.unit_order_query_batch_size.get(), 512);

    assert_eq!(config.comic_archive_export.payload_batch_mib.get(), 8);

    assert_eq!(config.comic_archive_export.artifact_limit_mib.get(), 256);

    assert_eq!(config.comic_archive_export.concurrency.get(), 1);

    assert_eq!(config.invitation.member_ttl, Duration::from_hours(120));

    assert_eq!(config.invitation.assignment_ttl, Duration::from_hours(72));

    assert_eq!(config.effect.capacity.get(), 1024);

    assert_eq!(config.sched.shutdown_grace, Duration::from_secs(30));

    assert_eq!(config.subtree_delete.concurrency.get(), 2);

    assert_eq!(config.subtree_delete.timeout, Duration::from_mins(5));

    assert_eq!(config.subtree_delete.idle_interval, Duration::from_secs(5));

    assert_eq!(config.subtree_delete.retry_interval, Duration::from_secs(5));
}

// default_overrides(AppConfig)(positive): empty documents, sections and constructors agree.
#[test]
fn empty_and_partial_sections_use_complete_defaults() {
    assert_defaults(&AppConfig::default());

    for document in [
        "",
        "[http]\n[http.rate_limit]\n[image]\n[artwork]\n[effect]\n[sched]\n[subtree_delete]\n[cache]\n[repo]\n[invitation]",
        "[http]\nport = 8888\n[image]\npage_source_image_limit_mib = 25",
    ] {
        assert_defaults(&AppConfig::parse(document).unwrap());
    }
}

// explicit_overrides(AppConfig)(positive): only supplied values replace defaults.
#[test]
fn partial_overrides_reach_runtime_policy() {
    let config = AppConfig::parse(
        r#"
        [http]
        cors_origin = "http://localhost:5173"
        [http.rate_limit]
        burst = 7
        [image]
        page_source_image_limit_mib = 30
        [cache]
        unit_save_limit_mib = 2
        unit_save_token_budget = 32
        unit_save_ttl_secs = 23
        online_user_ttl_secs = 11
        [repo]
        subtree_sweep_batch_size = 3
        unit_order_query_batch_size = 2
        [invitation]
        member_ttl_secs = 13
        assignment_ttl_secs = 19
        [effect]
        queue_capacity = 3
        [sched]
        shutdown_grace_secs = 0
        [subtree_delete]
        concurrency = 3
        timeout_secs = 17
        idle_interval_secs = 2
        retry_interval_secs = 4
    "#,
    )
    .unwrap();

    assert_eq!(config.http.cors_origin.header(), "http://localhost:5173");

    assert_eq!(config.http.rate_limit.burst.get(), 7);

    assert_eq!(config.http.rate_limit.requests_per_second.get(), 20);

    assert_eq!(config.image.page_source_image_limit_mib, 30);

    assert_eq!(config.image.user_avatar_limit_mib, 1);

    assert_eq!(config.cache.unit_save_limit_bytes(), 2 * 1024 * 1024);

    assert_eq!(config.cache.unit_save_min_entry_weight(), 65_536);

    assert_eq!(config.cache.unit_save_ttl, Duration::from_secs(23));

    assert_eq!(config.cache.online_user_ttl, Duration::from_secs(11));

    assert_eq!(config.repo.subtree_sweep_batch_size.get(), 3);

    assert_eq!(config.repo.unit_order_query_batch_size.get(), 2);

    assert_eq!(config.invitation.member_ttl, Duration::from_secs(13));

    assert_eq!(config.invitation.assignment_ttl, Duration::from_secs(19));

    assert_eq!(config.effect.capacity.get(), 3);

    assert_eq!(config.sched.shutdown_grace, Duration::ZERO);

    let task = config.subtree_delete.task_config();

    assert_eq!(task.name, "subtree_delete");

    assert_eq!(task.concurrency.get(), 3);

    assert_eq!(task.timeout, Duration::from_secs(17));

    assert_eq!(task.idle_interval, Duration::from_secs(2));

    assert_eq!(task.retry_interval, Duration::from_secs(4));
}

// invalid_overrides(AppConfig)(negative): explicit invalid values never fall back.
#[test]
fn invalid_runtime_settings_are_rejected() {
    for document in [
        "[cache]\nunit_save_limit_mib = 0",
        "[cache]\nunit_save_limit_mib = 4294967296",
        "[cache]\nunit_save_token_budget = 0",
        "[cache]\nunit_save_ttl_secs = 0",
        "[cache]\nunit_save_ttl_secs = -1",
        "[cache]\nunit_save_ttl_secs = 31536000001",
        "[cache]\nunit_save_limit_mib = 4096\nunit_save_token_budget = 1",
        "[cache]\nonline_user_ttl_secs = 0",
        "[cache]\nonline_user_ttl_secs = -1",
        "[repo]\nsubtree_sweep_batch_size = 0",
        "[repo]\nunit_order_query_batch_size = 0",
        "[repo]\nsubtree_sweep_batch_size = 4294967296",
        "[repo]\nunit_order_query_batch_size = 4294967296",
        "[comic_archive_export]\npayload_batch_mib = 0",
        "[comic_archive_export]\nartifact_limit_mib = 0",
        "[comic_archive_export]\nconcurrency = 0",
        "[comic_archive_export]\npayload_limit_mib = 8",
        "[invitation]\nmember_ttl_secs = 0",
        "[invitation]\nassignment_ttl_secs = 0",
        "[invitation]\nmember_ttl_secs = 1000000000000",
        "[invitation]\nassignment_ttl_secs = 1000000000000",
        "[http.rate_limit]\nrequests_per_second = 0",
        "[http.rate_limit]\nburst = 0",
        "[http.rate_limit]\nrequests_per_second = 1000000001",
        "[effect]\nqueue_capacity = 0",
        "[image]\npage_source_image_limit_mib = 0",
        "[effect]\nqueue_capacity = 9223372036854775807",
        "[subtree_delete]\nconcurrency = 0",
        "[subtree_delete]\ntimeout_secs = 0",
        "[subtree_delete]\nidle_interval_secs = 0",
        "[subtree_delete]\nretry_interval_secs = 0",
        "[subtree_delete]\ntimeout_secs = 18446744073709551615",
        "[sched]\nshutdown_grace_secs = 18446744073709551615",
        "[sched]\nshutdown_grace_secs = -1",
        "[http]\nport = '8888'",
        "[http",
    ] {
        assert!(AppConfig::parse(document).is_err(), "{document}");
    }

    for origin in [
        "*",
        "null",
        "",
        "ftp://example.com",
        "https://example.com/",
        "https://user@example.com",
        "https://example.com/path",
        "https://example.com?q=1",
        "https://example.com#fragment",
    ] {
        let document = format!("[http]\ncors_origin = {origin:?}");

        assert!(AppConfig::parse(&document).is_err(), "{origin}");
    }
}

// upload_units(AppConfig)(positive): unit-bearing keys honor explicit limits.
#[test]
fn upload_unit_keys_preserve_overrides() {
    let config = AppConfig::parse(
        r"
        [image]
        user_avatar_limit_mib = 3
        team_avatar_limit_mib = 4
        comic_cover_limit_mib = 5
        page_source_image_limit_mib = 6
        [artwork]
        chapter_artwork_limit_mib = 128
    ",
    )
    .unwrap();

    assert_eq!(config.image.user_avatar_limit_mib, 3);

    assert_eq!(config.image.team_avatar_limit_mib, 4);

    assert_eq!(config.image.comic_cover_limit_mib, 5);

    assert_eq!(config.image.page_source_image_limit_mib, 6);

    assert_eq!(config.artwork.chapter_artwork_limit_mib, 128);
}

// unknown_keys(AppConfig)(negative): obsolete names and misspellings cannot silently select defaults.
#[test]
fn obsolete_and_unknown_configuration_keys_are_rejected() {
    for document in [
        "[image]\nuser_avatar_limit = 1",
        "[image]\nteam_avatar_limit = 1",
        "[image]\ncomic_cover_limit = 2",
        "[image]\npage_source_image_limit = 25",
        "[image]\npage_image_limit_mib = 25",
        "[image]\npage_artwork_image_limit_mib = 0",
        "[artwork]\nchapter_artwork_limit = 512",
        "[cache]\nonline_user_ttl = 600",
        "[repo]\nunknown = 1",
        "[invitation]\nmember_ttl = 432000",
        "[invitation]\nassignment_ttl = 259200",
        "[unknown]",
        "[http]\nunknown = 1",
        "[http.rate_limit]\nunknown = 1",
        "[effect]\nunknown = 1",
        "[sched]\nshutdown_grace = 30",
        "[subtree_delete]\ntimeout = 300",
    ] {
        assert!(AppConfig::parse(document).is_err(), "{document}");
    }
}

// optional_file(AppConfig)(positive/negative): only absence falls back; read and parse errors survive.
#[tokio::test]
async fn optional_file_preserves_real_errors() {
    let directory = std::env::temp_dir()
        .join(format!("poprako-config-{}", uuid::Uuid::new_v4()));

    tokio::fs::create_dir(&directory).await.unwrap();

    let path = directory.join("app_config.toml");

    assert_defaults(&AppConfig::from_file(&path).await.unwrap());

    tokio::fs::write(&path, "").await.unwrap();

    assert_defaults(&AppConfig::from_file(&path).await.unwrap());

    tokio::fs::write(&path, "[http]\nport = 9999")
        .await
        .unwrap();

    assert_eq!(AppConfig::from_file(&path).await.unwrap().http.port, 9999);

    tokio::fs::write(&path, "[effect]\nqueue_capacity = 0")
        .await
        .unwrap();

    assert!(AppConfig::from_file(&path).await.is_err());

    tokio::fs::write(&path, [0xff]).await.unwrap();

    assert!(AppConfig::from_file(&path).await.is_err());

    assert!(AppConfig::from_file(&directory).await.is_err());

    tokio::fs::remove_dir_all(directory).await.unwrap();
}

// checked_in_configs(AppConfig)(positive): local and deployment overrides remain valid.
#[test]
fn checked_in_configs_preserve_environment_specific_values() {
    let local =
        AppConfig::parse(include_str!("../../app_config.toml")).unwrap();

    let deployment = AppConfig::parse(include_str!(
        "../../deploy/poprako-server/app_config.toml"
    ))
    .unwrap();

    assert_eq!(local.http.host, "127.0.0.1");

    assert_eq!(local.image.page_source_image_limit_mib, 30);

    assert_eq!(deployment.http.host, "0.0.0.0");

    assert_eq!(deployment.image.page_source_image_limit_mib, 25);
}

#[test]
fn parses_toml_http_settings() {
    //
    let config = AppConfig::parse(
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit_mib = 1
            team_avatar_limit_mib = 1
            comic_cover_limit_mib = 2
            page_source_image_limit_mib = 25
        "#,
    )
    .unwrap();

    assert_eq!(config.http.host, "127.0.0.1");

    assert_eq!(config.http.port, 8888);

    assert_eq!(config.image.user_avatar_limit_mib, 1);

    assert_eq!(config.image.team_avatar_limit_mib, 1);

    assert_eq!(config.image.comic_cover_limit_mib, 2);

    assert_eq!(config.image.page_source_image_limit_mib, 25);

    assert_eq!(config.image.page_artwork_image_limit_mib, 25);

    assert_eq!(config.artwork.chapter_artwork_limit_mib, 512);
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
fn rejects_zero_image_mib_limits() {
    //
    let zero_limit_configs = [
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit_mib = 0
            team_avatar_limit_mib = 1
            comic_cover_limit_mib = 1
            page_source_image_limit_mib = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit_mib = 1
            team_avatar_limit_mib = 0
            comic_cover_limit_mib = 1
            page_source_image_limit_mib = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit_mib = 1
            team_avatar_limit_mib = 1
            comic_cover_limit_mib = 0
            page_source_image_limit_mib = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit_mib = 1
            team_avatar_limit_mib = 1
            comic_cover_limit_mib = 1
            page_source_image_limit_mib = 0
        "#,
    ];

    for config_content in zero_limit_configs {
        //
        assert!(AppConfig::parse(config_content).is_err());
    }
}

// artwork_config(AppConfig)(negative): custom limits are honored and invalid limits fail at startup.
#[test]
fn artwork_config_validates_custom_limits() {
    let config_prefix = r#"
        [http]
        host = "127.0.0.1"
        port = 8888
        [image]
        user_avatar_limit_mib = 1
        team_avatar_limit_mib = 1
        comic_cover_limit_mib = 2
        page_source_image_limit_mib = 25
    "#;

    let custom = AppConfig::parse(&format!(
        "{config_prefix}\n[artwork]\nchapter_artwork_limit_mib = 128"
    ))
    .unwrap();

    assert_eq!(custom.artwork.chapter_artwork_limit_mib, 128);

    for limit in [0, u64::MAX] {
        assert!(
            AppConfig::parse(&format!(
                "{config_prefix}\n[artwork]\
                    \nchapter_artwork_limit_mib = {limit}"
            ))
            .is_err()
        );
    }
}
