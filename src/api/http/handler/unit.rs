//! Unit handlers: list and save page unit sequences.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;
#[allow(unused_imports)]
use crate::api::http::result::{Accept as _, HttpBody, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::unit::{
    ListPageUnitInfosParams, ListPageUnitInfosPayload, SavePageUnitsParams,
    SavePageUnitsPayload,
};
use crate::model::user::UserToken;
use crate::usecase;

/// `GET /api/v1/pages/{page_id}/units` — list units under a page.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/pages/{page_id}/units",
    tag = "units",
    params(("page_id" = String, Path, description = "Page ID")),
    responses(
        (status = 200, description = "Units listed", body = HttpBody<ListPageUnitInfosPayload>),
        (status = 403, description = "No permission to list units in this page"),
        (status = 404, description = "Page not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ListPageUnitInfosPayload> {
    //
    let params = ListPageUnitInfosParams { page_id };

    usecase::unit::list_infos((harn.repo(),), user_token, params)
        .await?
        .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/units/save` — save unit opers.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/units/save",
    tag = "units",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = SavePageUnitsParams,
    responses(
        (status = 200, description = "Units saved", body = HttpBody<SavePageUnitsPayload>),
        (status = 422, description = "Path id does not match body page id or diff page id"),
        (status = 403, description = "No permission to save units in this page"),
        (status = 422, description = "Invalid unit oper"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn save_infos(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<SavePageUnitsParams>,
) -> HttpResult<SavePageUnitsPayload> {
    //
    ensure_path_matches_body_id(&page_id, &params.page_id)?;

    ensure_path_matches_body_id(&page_id, &params.diff.page_id)?;

    usecase::unit::save((harn.drive(), harn.repo(),), user_token, params)
        .await?
        .accept(StatusCode::OK)
}
