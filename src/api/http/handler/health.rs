//! Health-check handler.

use std::net::SocketAddr;

use axum::Json;
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use tracing::instrument;

use crate::api::http::shared::prometheus::render_detailed_metrics;
use crate::api::http::shared::{MetricTotal, read_total};

#[cfg(test)]
// Health-check test fixtures stay isolated to this module.
mod tests;

/// `GET /api/health` — returns recent HTTP metrics to loopback callers.
///
/// Available in both debug and release builds. Non-loopback requests get a
/// body-less `404` so the endpoint is not advertised externally.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (
            status = 200,
            description = "Service is running normally (loopback only)",
            body = MetricTotal,
        ),
        (status = 404, description = "Not found (non-loopback request)"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn check_health(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<MetricTotal>, StatusCode> {
    //
    if !is_loopback(addr) {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(read_total()))
}

/// `GET /api/health/detailed-metrics` — renders Prometheus metrics locally.
///
/// Non-loopback requests receive a body-less `404` so the endpoint is not
/// advertised externally.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/health/detailed-metrics",
    tag = "health",
    responses(
        (
            status = 200,
            description = "Prometheus metrics (loopback only)",
            content_type = "text/plain",
        ),
        (status = 404, description = "Not found (non-loopback request)"),
        (status = 503, description = "Metrics recorder unavailable"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn detailed_metrics(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response, StatusCode> {
    //
    if !is_loopback(addr) {
        return Err(StatusCode::NOT_FOUND);
    }

    let Some(metrics_text) = render_detailed_metrics() else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    Ok((
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static(
                "text/plain; version=0.0.4; charset=utf-8",
            ),
        )],
        metrics_text,
    )
        .into_response())
}

// Returns true only for loopback addresses, restricting internal status endpoints to localhost.
fn is_loopback(addr: SocketAddr) -> bool {
    addr.ip().is_loopback()
}
