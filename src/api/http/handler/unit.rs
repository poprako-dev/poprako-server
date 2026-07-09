//! Unit handlers: list and save page unit sequences.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;

use tracing::instrument;

use crate::api::http::handler::util::{ensure_path_matches_body_id, Pagination};
use crate::api::http::result::{Accept as _, HttpBody, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::unit::{
    ListPageUnitInfosData, ListPageUnitInfosVal, SavePageUnitsData,
    SavePageUnitsVal,
};
use crate::model::user::UserToken;
use crate::usecase;

/// `GET /api/v1/pages/{page_id}/units` — list units under a page.
#[utoipa::path(
    get,
    path = "/api/v1/pages/{page_id}/units",
    tag = "units",
    params(("page_id" = String, Path, description = "Page ID"), Pagination),
    responses(
        (status = 200, description = "Units listed", body = HttpBody<ListPageUnitInfosVal>),
        (status = 403, description = "No permission to list units in this page"),
        (status = 404, description = "Page not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(pagination): Query<Pagination>,
) -> HttpResult<ListPageUnitInfosVal> {
    let data = ListPageUnitInfosData {
        page_id,
        offset: pagination.offset,
        limit: pagination.limit,
    };

    usecase::unit::list_infos(harn.repo(), user_token, data)
        .await?
        .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/units/save` — save unit opers.
#[utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/units/save",
    tag = "units",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = SavePageUnitsData,
    responses(
        (status = 200, description = "Units saved", body = HttpBody<SavePageUnitsVal>),
        (status = 422, description = "Path id does not match body page id or diff page id"),
        (status = 403, description = "No permission to save units in this page"),
        (status = 422, description = "Invalid unit oper"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn save_infos(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<SavePageUnitsData>,
) -> HttpResult<SavePageUnitsVal> {
    ensure_path_matches_body_id(&page_id, &data.page_id)?;

    ensure_path_matches_body_id(&page_id, &data.diff.page_id)?;

    usecase::unit::save_infos(harn.drive(), harn.repo(), user_token, data)
        .await?
        .accept(StatusCode::OK)
}
