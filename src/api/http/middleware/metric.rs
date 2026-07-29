//! HTTP response metric middleware.

use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::http::shared::record_response;

#[cfg(test)]
mod tests;

/// Records the response status and matched route template.
pub async fn record_response_metric(request: Request, next: Next) -> Response {
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
