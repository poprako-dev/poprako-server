use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::http::StatusCode;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpError;
use crate::api::http::result::HttpResult;

#[utoipa::path(
    get,
    path = "/check-health",
    tag = "health",
    responses(
        (status = 200, description = "Service is running normally (only responds to loopback)", body = str),
        (status = 404, description = "Not found (non-loopback request)", body = HttpError)
    )
)]
pub async fn check_health(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> HttpResult<&'static str> {
    if !addr.ip().is_loopback() {
        return Err(HttpError::not_found());
    }

    "PopRaKo-R running normally".accept(StatusCode::OK)
}
