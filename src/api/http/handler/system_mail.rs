//! System mail handlers: list and mark-read.

use axum::Json;
use axum::extract::Extension;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;

use tracing::instrument;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpNoContent;
use crate::api::http::result::HttpResult;
use crate::api::http::result::no_content;
use crate::api::http::state::AppHarn;
use crate::data::system_mail::{ListSystemMailData, MarkSystemMailsReadData, SystemMailVal};
use crate::model::user::UserToken;
use crate::usecase;

/// `GET /api/v1/system-mails` — list the current user's system mails.
#[utoipa::path(
    get,
    path = "/api/v1/system-mails",
    tag = "system-mails",
    params(ListSystemMailData),
    responses(
        (status = 200, description = "System mails listed", body = Vec<SystemMailVal>),
        (status = 401, description = "Authentication required"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(data): Query<ListSystemMailData>,
) -> HttpResult<Vec<SystemMailVal>> {
    let infos = usecase::system_mail::list_infos(harn.repo(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}

/// `POST /api/v1/system-mails/mark-read` — mark a batch of mails as read.
#[utoipa::path(
    post,
    path = "/api/v1/system-mails/mark-read",
    tag = "system-mails",
    request_body = MarkSystemMailsReadData,
    responses(
        (status = 204, description = "Mails marked as read"),
        (status = 403, description = "One or more mails do not belong to the user"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn mark_read(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<MarkSystemMailsReadData>,
) -> HttpNoContent {
    usecase::system_mail::mark_read(harn.repo(), user_token, data.ids).await?;

    no_content()
}
