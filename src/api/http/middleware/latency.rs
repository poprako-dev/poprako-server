//! Request latency logging middleware.

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// `from_fn` handler that logs the elapsed wall-clock time of each request.
pub async fn log_latency(request: Request, next: Next) -> Response {
    let start = Instant::now();

    let method = request.method().clone();

    let uri = request.uri().clone();

    let response = next.run(request).await;

    let duration = start.elapsed();

    tracing::info!(
        method = %method,
        uri = %uri,
        latency_ms = duration.as_secs_f64() * 1000.0,
        "request latency",
    );

    response
}
