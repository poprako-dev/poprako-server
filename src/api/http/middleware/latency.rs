//! Request latency logging middleware.

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

// `from_fn` handler that logs the elapsed wall-clock time of each request.
async fn log_latency(req: Request, next: Next) -> Response {
    //
    let start = Instant::now();

    let method = req.method().clone();

    let uri = req.uri().clone();

    let resp = next.run(req).await;

    let duration = start.elapsed();

    tracing::info!(
        method = %method,
        uri = %uri,
        latency_ms = duration.as_secs_f64() * 1000.0,
        "request latency",
    );

    resp
}
