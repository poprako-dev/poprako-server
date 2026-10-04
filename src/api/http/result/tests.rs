// http_body_serializes_success_envelope(HttpBody)(positive): emits code zero and data.

use super::*;

use poprako_obj_dept::rest::ObjDeptError;
use serde_json::json;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::part::auth::TokenAuth as _;
use crate::part_impl::auth::jwt_impl::JwtAuth;
use crate::test_util::http_logging::capture_request;

#[test]
fn http_body_serializes_success_envelope() {
    //
    let http_body =
        HttpBody::new(StatusCode::CREATED, json!({ "id": "comic_1" }));

    let serialized =
        serde_json::to_value(http_body).expect("http body serializes");

    assert_eq!(
        serialized,
        json!({
            "code": 0,
            "data": {
                "id": "comic_1",
            },
        }),
    );
}

#[test]
fn retryable_error_maps_to_conflict() {
    let http_error = HttpError::from(BaseError::Retryable {
        msg: "retry request".to_string(),
    });

    assert_eq!(http_error.status, StatusCode::CONFLICT);
    assert_eq!(http_error.code.get(), 8);
    assert_eq!(http_error.msg.as_deref(), Some("retry request"));

    assert_eq!(
        serde_json::to_value(http_error).expect("HTTP error serializes"),
        json!({"code": 8, "message": "retry request"}),
    );
}

#[test]
fn unavailable_error_maps_to_service_unavailable() {
    let http_error = HttpError::from(BaseError::Unavailable {
        msg: "driver detail".to_string(),
    });

    assert_eq!(http_error.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(http_error.code.get(), 9);
    assert_eq!(
        http_error.msg.as_deref(),
        Some(trl("error-unavailable").as_str()),
    );
}

#[test]
fn obj_dept_unavailable_error_maps_to_service_unavailable() {
    let base_error = BaseError::from(ObjDeptError::Unavailable {
        msg: "object database capacity unavailable".to_string(),
    });

    let http_error = HttpError::from(base_error);

    assert_eq!(http_error.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(http_error.code.get(), 9);
    assert_eq!(
        http_error.msg.as_deref(),
        Some(trl("error-unavailable").as_str()),
    );
}

#[tokio::test]
async fn application_client_errors_warn_at_source_without_http_duplicates() {
    for (variant, status, code) in [
        (ExpectedVariant::Args, StatusCode::UNPROCESSABLE_ENTITY, 2),
        (ExpectedVariant::Auth, StatusCode::UNAUTHORIZED, 3),
        (ExpectedVariant::Perm, StatusCode::FORBIDDEN, 4),
    ] {
        let router = Router::new().route(
            "/error",
            get(move || async move {
                Err::<(), _>(HttpError::from(BaseError::expected(
                    variant,
                    "source diagnostic".into(),
                )))
            }),
        );

        let request = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let (actual, body, logs) = capture_request(router, request).await;

        assert_eq!(actual, status);

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap()["code"],
            code
        );

        assert!(logs.contains("source diagnostic"), "{logs}");

        assert!(logs.contains("source_file="), "{logs}");

        assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");
    }
}

#[tokio::test]
async fn sdk_client_errors_keep_original_diagnostics_at_warning_level() {
    for (uri, status, reason) in [
        ("/auth", StatusCode::UNAUTHORIZED, "InvalidToken"),
        ("/conflict", StatusCode::CONFLICT, "serialization failure"),
        (
            "/mismatch",
            StatusCode::UNPROCESSABLE_ENTITY,
            "path id does not match body id",
        ),
    ] {
        let router = Router::new()
            .route("/auth", get(|| async {
                let auth = JwtAuth::new("test-signing-secret", 1).unwrap();

                let source = auth.verify_token("private-invalid-token").err().unwrap();

                Err::<(), _>(HttpError::from(source))
            }))
            .route("/conflict", get(|| async {
                let source = DieselError::DatabaseError(DatabaseErrorKind::SerializationFailure, Box::new("conflicting write".to_owned()));

                Err::<(), _>(HttpError::from(crate::shared::result::diesel(source)))
            }))
            .route("/mismatch", get(|| async {
                crate::api::http::handler::util::ensure_path_matches_body_id("one", "two")
            }));

        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();

        let (actual, _, logs) = capture_request(router, request).await;

        assert_eq!(actual, status);

        assert!(logs.contains(reason), "{logs}");

        assert!(!logs.contains("private-invalid-token"), "{logs}");

        assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");
    }
}
