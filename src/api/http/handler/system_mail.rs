//! System mail handlers: list and mark-read.
use axum::extract::{Extension, Json, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::system_mail::{
    ListSystemMailInfosQuery, MarkSystemMailReadBody, SystemMailInfoObj,
};
use crate::api::http::result::{
    AcceptInto as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::usecase::system_mail as system_mail_usecase;

/// `GET /api/v1/system-mails` — list the current user's system mails.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/system-mails",
    tag = "system-mails",
    params(ListSystemMailInfosQuery),
    responses(
        (status = 200, description = "System \
        mails listed", body = HttpBody<Vec<SystemMailInfoObj>>),
        (status = 401, description = "Authentication required"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ListSystemMailInfosQuery>,
) -> HttpResult<Vec<SystemMailInfoObj>> {
    //
    Ok(system_mail_usecase::list_infos(
        (harn.repo(),),
        user_token,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::OK))
}

/// `POST /api/v1/system-mails/mark-read` — mark a batch of mails as read.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/system-mails/mark-read",
    tag = "system-mails",
    request_body = MarkSystemMailReadBody,
    responses(
        (status = 204, description = "Mails marked as read"),
        (status = 403, description = "One or more mails do not belong to the \
        user"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_read(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkSystemMailReadBody>,
) -> HttpNoContent {
    //
    system_mail_usecase::mark_read((harn.repo(),), user_token, instr.ids)
        .await?;

    Ok(no_content())
}
