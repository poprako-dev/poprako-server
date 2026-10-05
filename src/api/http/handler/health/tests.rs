#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::routing::get;

use crate::test_util::http_logging::capture_request;

// is_loopback(is_loopback)(positive): IPv4 and IPv6 loopback callers are accepted.
// is_loopback(is_loopback)(negative): non-loopback callers are rejected.

#[test]
fn is_loopback_accepts_only_loopback_addresses() {
    //
    let ipv4_loopback = SocketAddr::from(([127, 0, 0, 1], 8080));

    let ipv6_loopback = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 8080));

    let public_addr = SocketAddr::from(([203, 0, 113, 1], 8080));

    assert!(is_loopback(ipv4_loopback));

    assert!(is_loopback(ipv6_loopback));

    assert!(!is_loopback(public_addr));
}

#[tokio::test]
async fn private_health_endpoints_warn_on_non_loopback_rejections() {
    for uri in ["/health", "/metrics"] {
        let router = Router::new()
            .route("/health", get(check_health))
            .route("/metrics", get(detailed_metrics));

        let request = Request::builder()
            .uri(uri)
            .extension(ConnectInfo(SocketAddr::from(([203, 0, 113, 1], 8080))))
            .body(Body::empty())
            .unwrap();

        let (status, body, logs) = capture_request(router, request).await;

        assert_eq!(status, StatusCode::NOT_FOUND);

        assert!(body.is_empty());

        assert!(logs.contains("requires a loopback caller"), "{logs}");

        assert_eq!(logs.matches(" WARN ").count(), 1, "{logs}");
    }
}
