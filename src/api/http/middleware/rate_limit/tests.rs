#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use std::num::NonZeroU32;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use tower::ServiceExt as _;

use super::*;

// Compose multiple routes with the same quota, as the production router does.
fn router() -> Router {
    let config = RateLimitConfig {
        requests_per_second: NonZeroU32::MIN,
        burst: NonZeroU32::new(2).unwrap(),
    };

    Router::new()
        .route("/first", get(|| async { StatusCode::OK }))
        .route("/second", get(|| async { StatusCode::OK }))
        .layer(from_fn_with_state(new(config), rate_limit))
}

async fn request(router: Router, path: &str) -> StatusCode {
    let request = Request::builder().uri(path).body(Body::empty()).unwrap();

    router.oneshot(request).await.unwrap().status()
}

// shared_quota(rate_limit)(positive/negative): routes and clones share the configured burst.
#[tokio::test]
async fn routes_and_clones_share_quota_but_applications_are_independent() {
    let first = router();

    let second = router();

    assert_eq!(request(first.clone(), "/first").await, StatusCode::OK);

    assert_eq!(request(first.clone(), "/second").await, StatusCode::OK);

    assert_eq!(
        request(first, "/first").await,
        StatusCode::TOO_MANY_REQUESTS
    );

    assert_eq!(request(second, "/first").await, StatusCode::OK);
}
