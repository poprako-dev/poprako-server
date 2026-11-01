//! Health-check handler.

#[cfg(test)]
// Health-check test fixtures stay isolated to this module.
mod tests;

use std::net::SocketAddr;

use axum::body::Bytes;
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, StatusCode, header};
use tracing::instrument;

use crate::api::http::result::{HttpBody, HttpError, HttpRawBody, HttpResult};

#[cfg(feature = "swagger")]
use crate::api::http::shared::metric::MetricTotal;

use crate::api::http::shared::metric::read_total;
use crate::api::http::shared::prometheus::render_detailed_metrics;

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
) -> HttpResult<HttpRawBody> {
    //
    if !is_loopback(addr) {
        //
        tracing::warn!(
            status = 404,
            error_variant = "HealthLoopbackRequired",
            "health endpoint requires a loopback caller",
        );

        return Ok(HttpBody::new(
            StatusCode::NOT_FOUND,
            HttpRawBody::new(Bytes::new()),
        ));
    }

    let body = serde_json::to_vec(&read_total()).map_err(|error| {
        //
        tracing::error!(
            operation = "serialize_health_metrics",
            sdk_error = ?error,
            "JSON SDK health metrics error",
        );

        HttpError::internal()
    })?;

    Ok(
        HttpBody::new(StatusCode::OK, HttpRawBody::new(body)).with_header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        ),
    )
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
) -> HttpResult<HttpRawBody> {
    //
    if !is_loopback(addr) {
        //
        tracing::warn!(
            status = 404,
            error_variant = "MetricsLoopbackRequired",
            "metrics endpoint requires a loopback caller",
        );

        return Ok(HttpBody::new(
            StatusCode::NOT_FOUND,
            HttpRawBody::new(Bytes::new()),
        ));
    }

    let Some(metrics_text) = render_detailed_metrics() else {
        //
        tracing::warn!(
            status = 503,
            error_variant = "Unavailable",
            error_message = "metrics recorder unavailable",
            "metrics recorder unavailable",
        );

        return Ok(HttpBody::new(
            StatusCode::SERVICE_UNAVAILABLE,
            HttpRawBody::new(Bytes::new()),
        ));
    };

    Ok(
        HttpBody::new(StatusCode::OK, HttpRawBody::new(metrics_text))
            .with_header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(
                    "text/plain; version=0.0.4; charset=utf-8",
                ),
            ),
    )
}

// Returns true only for loopback addresses, restricting internal status endpoints to localhost.
const fn is_loopback(addr: SocketAddr) -> bool {
    addr.ip().is_loopback()
}
