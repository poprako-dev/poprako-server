//! HTTP response metric middleware.

use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::http::shared::metric::record_response;

#[cfg(test)]
mod tests {
    use super::*;

    use axum::Router;
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use tower::ServiceExt as _;

    use crate::api::http::shared::metric::read_total;

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
}

/// Records the response status and matched route template.
pub(crate) async fn record_response_metric(
    request: Request,
    next: Next,
) -> Response {
    //
    let start = Instant::now();

    let method = request.method().to_string();

    let matched_path = request.extensions().get::<MatchedPath>().cloned();

    let response = next.run(request).await;

    let latency = start.elapsed();

    record_response(&response, matched_path.as_ref(), latency);

    let route = matched_path
        .as_ref()
        .map(MatchedPath::as_str)
        .unwrap_or("<unmatched>");

    let status = response.status().as_u16().to_string();

    metrics::counter!(
        "http_responses",
        "method" => method.clone(),
        "route" => route.to_owned(),
        "status" => status.clone(),
    )
    .increment(1);

    metrics::histogram!(
        "http_request_duration_seconds",
        "method" => method,
        "route" => route.to_owned(),
        "status" => status,
    )
    .record(latency.as_secs_f64());

    response
}
