#![allow(
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;
use crate::config::http::CorsOrigin;

use axum::Router;
use axum::body::Body;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS,
    ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
};
use axum::http::{HeaderValue, Method, Request, StatusCode};
use axum::routing::get;
use tower::ServiceExt as _;

#[tokio::test]
async fn cors_handles_web_client_preflight_before_api_authorization() {
    //
    let router = Router::new()
        .route(
            "/api/v1/system-mails",
            get(|| async { StatusCode::NO_CONTENT }),
        )
        .layer(cors(CorsOrigin::default().header()));

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/v1/system-mails")
        .header(ORIGIN, "https://poprako.com")
        .header(ACCESS_CONTROL_REQUEST_METHOD, Method::GET.as_str())
        .header(
            ACCESS_CONTROL_REQUEST_HEADERS,
            "authorization, content-type",
        )
        .body(Body::empty())
        .expect("CORS preflight request should be valid");

    let response = router
        .oneshot(request)
        .await
        .expect("CORS preflight response should be valid");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&HeaderValue::from_static("https://poprako.com")),
    );
    assert!(
        response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_METHODS)
            .is_some_and(|header_value| header_value
                .as_bytes()
                .contains(&b'G'))
    );
    assert!(
        response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_HEADERS)
            .is_some_and(|header_value| {
                let header_value = header_value.as_bytes();
                let has_authorization =
                    header_value.windows("authorization".len()).any(|window| {
                        window.eq_ignore_ascii_case(b"authorization")
                    });
                let has_content_type = header_value
                    .windows("content-type".len())
                    .any(|window| window.eq_ignore_ascii_case(b"content-type"));

                has_authorization && has_content_type
            })
    );
}

// configured_origin(cors)(positive/negative): only the injected origin receives browser access.
#[tokio::test]
async fn configured_origin_controls_preflight_access() {
    let origin = CorsOrigin::try_from("http://localhost:5173".to_owned())
        .expect("local browser origin should be valid");

    let router = Router::new()
        .route("/known", get(|| async { StatusCode::OK }))
        .layer(cors(origin.header()));

    for (request_origin, allowed) in [
        ("http://localhost:5173", true),
        ("https://poprako.com", false),
        ("https://unrelated.example", false),
    ] {
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/known")
            .header(ORIGIN, request_origin)
            .header(ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .expect("preflight request should be valid");

        let response = router
            .clone()
            .oneshot(request)
            .await
            .expect("preflight should succeed");

        let permits_origin = response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_some_and(|value| value == request_origin);

        assert_eq!(permits_origin, allowed);

        assert_eq!(
            response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN),
            allowed
        );
    }
}
