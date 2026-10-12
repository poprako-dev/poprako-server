#![allow(
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::middleware::map_response;
use axum::routing::get;
use tower::ServiceExt as _;

use super::*;
use crate::api::http::middleware::cors::cors;

#[tokio::test]
async fn cache_policy_overrides_existing_headers_and_preserves_response() {
    for status in [
        StatusCode::OK,
        StatusCode::UNAUTHORIZED,
        StatusCode::TOO_MANY_REQUESTS,
        StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        let router = Router::new()
            .route(
                "/known",
                get(move || async move {
                    (status, [(CACHE_CONTROL, "public, max-age=3600")], "body")
                }),
            )
            .layer(map_response(disable_cache));

        let request = Request::builder()
            .uri("/known")
            .body(Body::empty())
            .expect("Request should be valid");

        let response = router
            .oneshot(request)
            .await
            .expect("Request should succeed");

        assert_eq!(response.status(), status);

        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );

        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("Body should be readable");

        assert_eq!(body.as_ref(), b"body");
    }
}

#[tokio::test]
async fn cache_policy_covers_routing_rejections_and_cors_preflight() {
    for (uri, method, status) in [
        ("/missing", "GET", StatusCode::NOT_FOUND),
        ("/known", "POST", StatusCode::METHOD_NOT_ALLOWED),
        ("/known", "OPTIONS", StatusCode::OK),
    ] {
        let router = Router::new()
            .route("/known", get(|| async { StatusCode::OK }))
            .layer(cors())
            .layer(map_response(disable_cache));

        let request = Request::builder()
            .uri(uri)
            .method(method)
            .header("origin", "https://poprako.com")
            .header("access-control-request-method", "GET")
            .body(Body::empty())
            .expect("Request should be valid");

        let response = router
            .oneshot(request)
            .await
            .expect("Request should succeed");

        assert_eq!(response.status(), status);

        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
    }
}
