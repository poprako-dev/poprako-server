//! Routing failures must retain their status and warn at their source.

use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use tower::ServiceExt as _;

use super::*;
use crate::test_util::http_logging::capture_request;

#[tokio::test]
async fn route_and_method_rejections_record_one_correlated_warning() {
    for (uri, method, status, diagnostic) in [
        ("/missing", "GET", StatusCode::NOT_FOUND, "RouteNotFound"),
        (
            "/known",
            "POST",
            StatusCode::METHOD_NOT_ALLOWED,
            "MethodNotAllowed",
        ),
    ] {
        let router = Router::new()
            .route("/known", get(|| async { StatusCode::OK }))
            .fallback(not_found)
            .method_not_allowed_fallback(method_not_allowed);

        let request = Request::builder()
            .uri(uri)
            .method(method)
            .header("x-request-id", "routing-rejection")
            .body(Body::empty())
            .unwrap();

        let (actual, body, logs) = capture_request(router, request).await;

        assert_eq!(actual, status);

        assert!(body.is_empty());

        assert!(logs.contains(diagnostic), "{logs}");

        assert!(logs.contains("routing-rejection"), "{logs}");

        assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");
    }
}

#[tokio::test]
async fn method_rejection_preserves_allow_header() {
    let router = Router::new()
        .route("/known", get(|| async { StatusCode::OK }))
        .method_not_allowed_fallback(method_not_allowed);

    let request = Request::builder()
        .uri("/known")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    assert_eq!(response.headers()["allow"], "GET,HEAD");
}

#[tokio::test]
async fn protected_method_fallback_keeps_authorization_before_routing() {
    let router = Router::new()
        .route("/known", get(|| async { StatusCode::OK }))
        .method_not_allowed_fallback(method_not_allowed)
        .layer(from_fn(|_: Request<Body>, _: Next| async {
            crate::api::http::result::HttpError::from(
                crate::result::BaseError::expected(
                    crate::result::ExpectedVariant::Auth,
                    "authentication required".into(),
                ),
            )
        }))
        .method_not_allowed_fallback(method_not_allowed);

    let request = Request::builder()
        .uri("/known")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let (status, _, logs) = capture_request(router, request).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);

    assert!(logs.contains("authentication required"), "{logs}");

    assert!(!logs.contains("MethodNotAllowed"), "{logs}");

    assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");
}
