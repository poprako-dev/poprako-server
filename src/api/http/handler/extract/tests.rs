//! HTTP rejection logging must work with the production INFO filter.

use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Extension, Json, Path, Query};
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum_extra::extract::Query as MultiQuery;
use serde::Deserialize;

use crate::api::http::result::HttpError;
use crate::data::instr::unit::UnitEditInstr;
use crate::result::{BaseError, ExpectedVariant};
use crate::test_util::http_logging::capture_request;

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
        .route("/extension", get(|Extension(_): Extension<u32>| async { StatusCode::OK }))
        .route("/string", post(|_: String| async { StatusCode::OK }))
        .route("/bytes", post(|_: axum::body::Bytes| async { StatusCode::OK }))
        .layer(DefaultBodyLimit::max(128))
}

#[tokio::test]
async fn invalid_unit_patch_records_framework_rejection_before_handler_runs() {
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

    assert!(logs.contains("error_origin=\"axum\""), "{logs}");

    assert!(logs.contains("JsonDataError"), "{logs}");

    assert_eq!(logs.matches("axum::rejection:").count(), 1, "{logs}");

    assert!(!logs.contains("missing field"), "{logs}");

    assert!(logs.contains("unit-save-rejection"), "{logs}");

    assert!(!logs.contains("private-translation"), "{logs}");
}

#[tokio::test]
async fn native_request_extractors_record_concrete_rejections_without_payloads()
{
    for (uri, method, content_type, payload, status, reason, variant) in [
        (
            "/json",
            "POST",
            "application/json",
            "{",
            400,
            "EOF while parsing",
            "JsonSyntaxError",
        ),
        (
            "/json",
            "POST",
            "application/json",
            r#"{"id":"private-sentinel"}"#,
            422,
            "invalid type",
            "JsonDataError",
        ),
        (
            "/json",
            "POST",
            "text/plain",
            "{}",
            415,
            "application/json",
            "MissingJsonContentType",
        ),
        (
            "/query?id=private-sentinel",
            "GET",
            "text/plain",
            "",
            400,
            "invalid digit",
            "FailedToDeserializeQueryString",
        ),
        (
            "/multi?id=1&id=private-sentinel",
            "GET",
            "text/plain",
            "",
            400,
            "invalid digit",
            "axum_extra::extract::query::FailedToDeserializeQueryString",
        ),
        (
            "/path/private-sentinel",
            "GET",
            "text/plain",
            "",
            400,
            "Cannot parse",
            "FailedToDeserializePathParams",
        ),
        (
            "/extension",
            "GET",
            "text/plain",
            "",
            500,
            "Missing request extension",
            "MissingExtension",
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

        assert!(logs.contains(variant), "{logs}");

        assert!(logs.contains(&format!("status={status}")), "{logs}");

        assert!(logs.contains("error_origin=\"axum\""), "{logs}");

        assert!(logs.contains("extractor-rejection"), "{logs}");

        assert_eq!(logs.matches("axum::rejection:").count(), 1, "{logs}");

        assert!(!logs.contains("private-credential"), "{logs}");

        // Existing request spans retain paths; rejection fields exclude client values.
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
async fn body_extractors_record_size_rejections_and_retain_413() {
    for uri in ["/json", "/string", "/bytes"] {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(" ".repeat(129)))
            .unwrap();

        let (status, _, logs) =
            capture_request(extractor_router(), request).await;

        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

        assert!(logs.contains("LengthLimitError"), "{logs}");

        assert_eq!(logs.matches("axum::rejection:").count(), 1, "{logs}");
    }
}

#[tokio::test]
async fn invalid_utf8_and_failed_body_reads_record_native_rejections() {
    let failed_read = Body::from_stream(futures_util::stream::once(async {
        Err::<axum::body::Bytes, _>(std::io::Error::other("private-body-error"))
    }));

    for (uri, body, expected_status, variant) in [
        (
            "/string",
            Body::from(vec![0xff]),
            StatusCode::BAD_REQUEST,
            "InvalidUtf8",
        ),
        (
            "/bytes",
            failed_read,
            StatusCode::BAD_REQUEST,
            "UnknownBodyError",
        ),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("x-request-id", "body-rejection")
            .body(body)
            .unwrap();

        let (status, _, logs) =
            capture_request(extractor_router(), request).await;

        assert_eq!(status, expected_status);

        assert!(logs.contains(variant), "{logs}");

        assert!(logs.contains("body-rejection"), "{logs}");

        assert!(!logs.contains("private-body-error"), "{logs}");

        assert_eq!(logs.matches("axum::rejection:").count(), 1, "{logs}");
    }
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

    assert!(!logs.contains("axum::rejection:"), "{logs}");
}

#[tokio::test]
async fn business_and_framework_422_have_distinct_diagnostics() {
    let router = Router::new().route(
        "/input",
        post(|Json(_): Json<Input>| async {
            HttpError::from(BaseError::expected(
                ExpectedVariant::Args,
                "business validation failed".into(),
            ))
        }),
    );

    for (payload, framework) in
        [(r#"{"id":"private-input"}"#, true), (r#"{"id":1}"#, false)]
    {
        let request = Request::builder()
            .method("POST")
            .uri("/input")
            .header("content-type", "application/json")
            .header("x-request-id", "same-422-route")
            .body(Body::from(payload))
            .unwrap();

        let (status, _, logs) = capture_request(router.clone(), request).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        assert_eq!(logs.contains("error_origin=\"axum\""), framework, "{logs}");

        assert_eq!(
            logs.contains("business validation failed"),
            !framework,
            "{logs}"
        );

        assert!(logs.contains("same-422-route"), "{logs}");

        assert!(!logs.contains("private-input"), "{logs}");
    }
}
