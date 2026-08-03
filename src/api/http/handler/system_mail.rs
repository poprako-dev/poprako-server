//! System mail handlers: list and mark-read.

use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::data::instr::system_mail::{
    ListSystemMailInfosInstr, MarkSystemMailReadInstr,
};
use crate::data::view::system_mail::SystemMailInfoView;

#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::usecase;

/// `GET /api/v1/system-mails` — list the current user's system mails.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/system-mails",
    tag = "system-mails",
    params(ListSystemMailInfosInstr),
    responses(
        (status = 200, description = "System mails listed", body = HttpBody<Vec<SystemMailInfoView>>),
        (status = 401, description = "Authentication required"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ListSystemMailInfosInstr>,
) -> HttpResult<Vec<SystemMailInfoView>> {
    usecase::system_mail::list_infos((harn.repo(),), user_token, instr)
        .await?
        .accept(StatusCode::OK)
}

/// `POST /api/v1/system-mails/mark-read` — mark a batch of mails as read.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/system-mails/mark-read",
    tag = "system-mails",
    request_body = MarkSystemMailReadInstr,
    responses(
        (status = 204, description = "Mails marked as read"),
        (status = 403, description = "One or more mails do not belong to the user"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_read(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkSystemMailReadInstr>,
) -> HttpNoContent {
    //
    usecase::system_mail::mark_read((harn.repo(),), user_token, instr.ids)
        .await?;

    no_content()
}
