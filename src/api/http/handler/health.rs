//! Health-check handler.

use std::net::SocketAddr;

use axum::Json;
use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::shared::metric::{MetricTotal, read_total};

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
    if !addr.ip().is_loopback() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(read_total()))
}
