use super::*;

use axum::Router;
use axum::body::Body;
use axum::http::StatusCode;
use axum::middleware::from_fn;
use axum::routing::get;
use tower::ServiceExt as _;

use crate::api::http::shared::read_total;

// record_response_metric(record_response_metric)(positive): dynamic paths use the matched route template.

#[tokio::test]
async fn records_matched_route_template() {
    //
    let router = Router::new()
        .route(
            "/metric-test/items/{item_id}",
            get(|| async { StatusCode::NO_CONTENT }),
        )
        .layer(from_fn(record_response_metric));

    let request = Request::builder()
        .uri("/metric-test/items/real-item-id")
        .body(Body::empty())
        .expect("request should be valid");

    let response = router
        .oneshot(request)
        .await
        .expect("metric test response should succeed");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let metric_total = read_total();

    assert_eq!(
        metric_total.by_path.get("/metric-test/items/{item_id}"),
        Some(&1),
    );

    assert!(
        !metric_total
            .by_path
            .contains_key("/metric-test/items/real-item-id")
    );
}
