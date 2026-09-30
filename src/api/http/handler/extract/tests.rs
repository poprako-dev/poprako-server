//! HTTP rejection logging must work with the production INFO filter.

use axum::Router;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use serde::Deserialize;

use super::{Json, MultiQuery, Path, Query};
use crate::data::instr::unit::UnitEditInstr;

#[derive(Deserialize)]
struct Input {
    id: u32,
}

#[derive(Deserialize)]
struct RepeatedInput {
    id: Vec<u32>,
}

fn extractor_router() -> Router {
    Router::new()
        .route("/json", post(|Json(value): Json<Input>| async move { value.id.to_string() }))
        .route("/query", get(|Query(value): Query<Input>| async move { value.id.to_string() }))
        .route("/multi", get(|MultiQuery(value): MultiQuery<RepeatedInput>| async move { value.id.len().to_string() }))
        .route("/path/{id}", get(|Path(id): Path<u32>| async move { id.to_string() }))
        .layer(DefaultBodyLimit::max(128))
}
use crate::test_util::http_logging::capture_request;

#[tokio::test]
async fn invalid_unit_patch_records_source_warning_before_handler_runs() {
    let router = Router::new().route(
        "/units/save",
        post(|Json(_): Json<Vec<UnitEditInstr>>| async { StatusCode::OK }),
    );

    let request = Request::builder()
        .method("POST")
        .uri("/units/save")
        .header("content-type", "application/json")
        .header("x-request-id", "unit-save-rejection")
        .body(Body::from(
            r#"[{"edit":"patch","id":"unit-1","translation":{"translated_text":"private-translation"}}]"#,
        ))
        .unwrap();

    let (status, body, logs) = capture_request(router, request).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    assert!(body.contains("missing field `type`"), "{body}");

    assert!(logs.contains("missing field `type`"), "{logs}");

    assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");

    assert!(logs.contains("unit-save-rejection"), "{logs}");

    assert!(!logs.contains("private-translation"), "{logs}");
}

#[tokio::test]
async fn all_request_extractors_warn_with_original_rejection_status() {
    for (uri, method, content_type, payload, status, reason) in [
        (
            "/json",
            "POST",
            "application/json",
            "{",
            400,
            "EOF while parsing",
        ),
        (
            "/json",
            "POST",
            "application/json",
            r#"{"id":"private-sentinel"}"#,
            422,
            "invalid type",
        ),
        ("/json", "POST", "text/plain", "{}", 415, "application/json"),
        (
            "/query?id=private-sentinel",
            "GET",
            "text/plain",
            "",
            400,
            "invalid digit",
        ),
        (
            "/multi?id=1&id=private-sentinel",
            "GET",
            "text/plain",
            "",
            400,
            "invalid digit",
        ),
        (
            "/path/private-sentinel",
            "GET",
            "text/plain",
            "",
            400,
            "Cannot parse",
        ),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", content_type)
            .header("authorization", "Bearer private-credential")
            .header("x-request-id", "extractor-rejection")
            .body(Body::from(payload))
            .unwrap();

        let (actual, body, logs) =
            capture_request(extractor_router(), request).await;

        assert_eq!(actual.as_u16(), status);

        assert!(body.contains(reason), "{body}");

        assert!(logs.contains(reason), "{logs}");

        assert!(logs.contains("extractor-rejection"), "{logs}");

        assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");

        assert!(!logs.contains("private-credential"), "{logs}");

        // The path remains in the request span; diagnostic fields must redact its value.
        let diagnostic = logs
            .split("HTTP request extraction rejected")
            .nth(1)
            .unwrap();

        assert!(
            !diagnostic
                .lines()
                .next()
                .unwrap()
                .contains("private-sentinel"),
            "{logs}"
        );

        if !uri.starts_with("/path/") {
            assert!(!logs.contains("private-sentinel"), "{logs}");
        }
    }
}

#[tokio::test]
async fn oversized_body_warns_and_retains_413() {
    let request = Request::builder()
        .method("POST")
        .uri("/json")
        .header("content-type", "application/json")
        .body(Body::from(" ".repeat(129)))
        .unwrap();

    let (status, _, logs) = capture_request(extractor_router(), request).await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

    assert!(logs.contains("length limit exceeded"), "{logs}");

    assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");
}

#[tokio::test]
async fn valid_repeated_queries_keep_every_value_without_warning() {
    let request = Request::builder()
        .uri("/multi?id=1&id=2")
        .body(Body::empty())
        .unwrap();

    let (status, body, logs) =
        capture_request(extractor_router(), request).await;

    assert_eq!(status, StatusCode::OK);

    assert_eq!(body, "2");

    assert!(!logs.contains(" WARN "), "{logs}");
}

#[test]
fn diagnostics_do_not_leak_input_containing_quote_delimiters() {
    for message in [
        "unknown variant `value`private-sentinel`, expected `create`",
        "unknown field `value`private-sentinel`, expected `id`",
        "Cannot parse `value`private-sentinel` to a `u32`",
        r#"invalid type: string "value\"private-sentinel", expected u32"#,
        r#"invalid type: string "private-sentinel unknown field injected", expected u32"#,
    ] {
        let safe = super::redact_diagnostic(message);

        assert!(!safe.contains("private-sentinel"), "{safe}");

        assert!(safe.contains("<REDACTED>"), "{safe}");
    }

    let missing = "[0].translation: missing field `type` at line 1 column 100";

    assert_eq!(super::redact_diagnostic(missing), missing);
}
