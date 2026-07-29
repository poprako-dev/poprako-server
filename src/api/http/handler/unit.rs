//! Unit handlers: list and save page unit sequences.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::unit::{
    ListPageUnitInfosParams, ListPageUnitInfosPayload, SavePageUnitEditsParams,
    UnitEditVal,
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

/// `POST /api/v1/pages/{page_id}/units/save` — save Unit edits.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/units/save",
    tag = "units",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = Vec<UnitEditVal>,
    responses(
        (status = 204, description = "Unit edits saved"),
        (status = 403, description = "No permission to save units in this page"),
        (status = 422, description = "Invalid Unit edit"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn save_infos(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(edits): Json<Vec<UnitEditVal>>,
) -> HttpNoContent {
    //
    let params = SavePageUnitEditsParams { page_id, edits };

    usecase::unit::save_edits((harn.drive(), harn.repo()), user_token, params)
        .await?;

    no_content()
}
