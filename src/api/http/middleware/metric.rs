//! HTTP response metric middleware.

#[cfg(test)]
// Metrics middleware test fixtures stay with the middleware module.
mod tests;

use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;

use crate::api::http::shared::metric::record_resp;

/// Records the response status and matched route template.
pub async fn record_resp_metric(req: Request, next: Next) -> Response {
    //
    let start = Instant::now();

    let method = req.method().to_string();

    let matched_path = req.extensions().get::<MatchedPath>().cloned();

    let resp = next.run(req).await;

    let latency = start.elapsed();

    record_resp(&resp, matched_path.as_ref(), latency);

    let route = matched_path
        .as_ref()
        .map_or("<unmatched>", MatchedPath::as_str);

    let status = resp.status().as_u16().to_string();

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

    resp
}
