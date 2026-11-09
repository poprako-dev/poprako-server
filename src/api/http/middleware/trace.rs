//! HTTP request correlation and tracing middleware.

#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, Request};
use axum::http::header::USER_AGENT;
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::{
    MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer,
};
use tower_http::trace::{
    DefaultOnFailure, HttpMakeClassifier, OnFailure as _, TraceLayer,
};
use tracing::Span;

/// Builds the request-ID propagation layer.
pub fn propagate_req_id() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::x_request_id()
}

/// Builds the request-ID generation layer.
pub fn set_req_id() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::x_request_id(MakeRequestUuid)
}

// Trace layer type used by request/response instrumentation.
type HttpTraceLayer = TraceLayer<
    HttpMakeClassifier,
    fn(&Request) -> Span,
    fn(&Request, &Span),
    fn(&Response, Duration, &Span),
    (),
    (),
    fn(ServerErrorsFailureClass, Duration, &Span),
>;

/// Builds the HTTP tracing layer with request correlation fields.
pub fn trace_req() -> HttpTraceLayer {
    //
    TraceLayer::new_for_http()
        .make_span_with(make_req_span as _)
        .on_request(record_req_started as _)
        .on_response(record_req_resp as _)
        .on_body_chunk(())
        .on_eos(())
        .on_failure(record_req_failure as _)
}

// Creates the top-level span for one HTTP request.
fn make_req_span(req: &Request) -> Span {
    //
    let req_id = req
        .headers()
        .get("x-request-id")
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or("invalid");

    let user_agent = req
        .headers()
        .get(USER_AGENT)
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or("unknown");

    let connect_info = req.extensions().get::<ConnectInfo<SocketAddr>>();

    let remote_addr = connect_info
        .map(|connect_info| connect_info.0)
        .map_or_else(|| "unknown".to_owned(), |addr| addr.to_string());

    tracing::info_span!(
        "request",
        req_id = %req_id,
        method = %req.method(),
        uri = req.uri().path(),
        http_ver = ?req.version(),
        user_agent = %user_agent,
        remote_addr = %remote_addr,
    )
}

// Records a received HTTP request.
fn record_req_started(req: &Request, _span: &Span) {
    //
    metrics::counter!(
        "http_requests_started",
        "method" => req.method().to_string(),
    )
    .increment(1);
}

// Records an HTTP response and its latency.
fn record_req_resp(resp: &Response, latency: Duration, _span: &Span) {
    //
    metrics::counter!(
        "http_requests",
        "status" => resp.status().as_u16().to_string(),
    )
    .increment(1);

    tracing::info!(
        status = resp.status().as_u16(),
        latency_millis = latency.as_secs_f64() * 1_000.0,
        "request completed",
    );
}

// Counts failures while leaving response-status diagnostics at their source.
fn record_req_failure(
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

    // Application and handler errors have already been recorded. Transport
    // errors without a response still need diagnostics at this boundary.
    if matches!(failure, ServerErrorsFailureClass::StatusCode(_)) {
        return;
    }

    DefaultOnFailure::new().on_failure(failure, latency, span);
}
