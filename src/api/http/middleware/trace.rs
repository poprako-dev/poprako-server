//! HTTP request correlation and tracing middleware.

use std::time::Duration;

use axum::extract::Request;
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::{
    MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer,
};
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use tracing::Span;

/// Builds the request-ID propagation layer.
pub fn propagate_request_id() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::x_request_id()
}

/// Builds the request-ID generation layer.
pub fn set_request_id() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::x_request_id(MakeRequestUuid)
}

type HttpTraceLayer = TraceLayer<
    HttpMakeClassifier,
    fn(&Request) -> Span,
    fn(&Request, &Span),
    fn(&Response, Duration, &Span),
    (),
    (),
    fn(ServerErrorsFailureClass, Duration, &Span),
>;

/// Builds the HTTP tracing layer with the request correlation fields.
pub fn trace_request() -> HttpTraceLayer {
    TraceLayer::new_for_http()
        .make_span_with(make_request_span as fn(&Request) -> Span)
        .on_request(record_request_started as fn(&Request, &Span))
        .on_response(record_request_response as fn(&Response, Duration, &Span))
        .on_body_chunk(())
        .on_eos(())
        .on_failure(
            record_request_failure
                as fn(ServerErrorsFailureClass, Duration, &Span),
        )
}

/// Creates the top-level span for one HTTP request.
fn make_request_span(request: &Request) -> Span {
    //
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or("invalid");

    tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
    )
}

/// Records a received HTTP request.
fn record_request_started(request: &Request, _span: &Span) {
    metrics::counter!(
        "http_requests_started_total",
        "method" => request.method().to_string(),
    )
    .increment(1);
}

/// Records an HTTP response and its latency.
fn record_request_response(
    response: &Response,
    _latency: Duration,
    _span: &Span,
) {
    metrics::counter!(
        "http_requests_total",
        "status" => response.status().as_u16().to_string(),
    )
    .increment(1);

    // metrics::histogram!("http_request_duration_seconds")
    //     .record(latency.as_secs_f64());
}

/// Records an HTTP server-error response.
fn record_request_failure(
    failure: ServerErrorsFailureClass,
    _latency: Duration,
    _span: &Span,
) {
    metrics::counter!(
        "http_request_failures_total",
        "class" => failure.to_string(),
    )
    .increment(1);
}
