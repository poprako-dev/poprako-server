//! HTTP request correlation and tracing middleware.

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, Request};
use axum::http::header::USER_AGENT;
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnFailure, HttpMakeClassifier, OnFailure as _, TraceLayer};
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
        .make_span_with(make_request_span as _)
        .on_request(record_request_started as _)
        .on_response(record_request_response as _)
        .on_body_chunk(())
        .on_eos(())
        .on_failure(record_request_failure as _)
}

/// Creates the top-level span for one HTTP request.
fn make_request_span(request: &Request) -> Span {
    //
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or("invalid");

    let user_agent = request
        .headers()
        .get(USER_AGENT)
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or("unknown");

    let remote_addr = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0)
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
        user_agent = %user_agent,
        remote_addr = %remote_addr,
    )
}

/// Records a received HTTP request.
fn record_request_started(request: &Request, _span: &Span) {
    metrics::counter!(
        "http_requests_started",
        "method" => request.method().to_string(),
    )
    .increment(1);
}

/// Records an HTTP response and its latency.
fn record_request_response(
    response: &Response,
    latency: Duration,
    _span: &Span,
) {
    //
    metrics::counter!(
        "http_requests",
        "status" => response.status().as_u16().to_string(),
    )
    .increment(1);

    tracing::info!(
        status = response.status().as_u16(),
        latency_millis = latency.as_secs_f64() * 1_000.0,
        "request completed",
    );
}

/// Records an HTTP server-error response.
fn record_request_failure(
    failure: ServerErrorsFailureClass,
    latency: Duration,
    span: &Span,
) {
    //
    metrics::counter!(
        "http_request_failures",
        "class" => failure.to_string(),
    )
    .increment(1);

    DefaultOnFailure::new().on_failure(failure, latency, span);
}
