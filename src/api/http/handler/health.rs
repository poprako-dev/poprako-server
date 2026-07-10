//! Health-check handler.

use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::http::StatusCode;

/// `GET /api/health` — returns `204` for loopback callers, `404` otherwise.
///
/// Available in both debug and release builds. Non-loopback requests get a
/// body-less `404` so the endpoint is not advertised externally.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 204, description = "Service is running normally (loopback only)"),
        (status = 404, description = "Not found (non-loopback request)"),
    ),
))]
pub async fn check_health(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<StatusCode, StatusCode> {
    if !addr.ip().is_loopback() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
