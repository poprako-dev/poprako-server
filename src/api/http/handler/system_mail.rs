//! System mail handlers: list and mark-read.

use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;

use tracing::instrument;

#[cfg(feature = "swagger-ui")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::system_mail_data;
use crate::model::user_model;
use crate::usecase;

/// `GET /api/v1/system-mails` — list the current user's system mails.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/system-mails",
    tag = "system-mails",
    params(system_mail_data::ListData),
    responses(
        (status = 200, description = "System mails listed", body = HttpBody<Vec<system_mail_data::Val>>),
        (status = 401, description = "Authentication required"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Query(data): Query<system_mail_data::ListData>,
) -> HttpResult<Vec<system_mail_data::Val>> {
    usecase::system_mail::list_infos(harn.repo(), user_token, data)
        .await?
        .accept(StatusCode::OK)
}

/// `POST /api/v1/system-mails/mark-read` — mark a batch of mails as read.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/system-mails/mark-read",
    tag = "system-mails",
    request_body = system_mail_data::MarkReadData,
    responses(
        (status = 204, description = "Mails marked as read"),
        (status = 403, description = "One or more mails do not belong to the user"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn mark_read(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<system_mail_data::MarkReadData>,
) -> HttpNoContent {
    //
    usecase::system_mail::mark_read(harn.repo(), user_token, data.ids).await?;

    no_content()
}
