//! Health-check handler.

use std::net::SocketAddr;

use axum::Json;
use axum::extract::ConnectInfo;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse as _, Response};
use tracing::instrument;

use crate::api::http::shared::metric::{MetricTotal, read_total};
use crate::api::http::shared::prometheus::render_detailed_metrics;

#[cfg(test)]
mod tests {
    use super::*;

    // is_loopback(is_loopback)(positive): IPv4 and IPv6 loopback callers are accepted.
    // is_loopback(is_loopback)(negative): non-loopback callers are rejected.

    #[test]
    fn is_loopback_accepts_only_loopback_addresses() {
        //
        let ipv4_loopback = SocketAddr::from(([127, 0, 0, 1], 8080));

        let ipv6_loopback = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 8080));

        let public_addr = SocketAddr::from(([203, 0, 113, 1], 8080));

        assert!(is_loopback(ipv4_loopback));

        assert!(is_loopback(ipv6_loopback));

        assert!(!is_loopback(public_addr));
    }
}

/// `GET /api/health` — returns recent HTTP metrics to loopback callers.
///
/// Available in both debug and release builds. Non-loopback requests get a
/// body-less `404` so the endpoint is not advertised externally.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
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
#[instrument(level = "info", err(Debug), skip_all)]
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
#[cfg_attr(feature = "swagger-ui", utoipa::path(
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
#[instrument(level = "info", err(Debug), skip_all)]
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

fn is_loopback(addr: SocketAddr) -> bool {
    addr.ip().is_loopback()
}
