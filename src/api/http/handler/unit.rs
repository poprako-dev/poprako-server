//! Unit handlers: list and save page unit sequences.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;
use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::unit::{
    ListPageUnitInfosInstr, SavePageUnitEditsInstr, UnitEditInstr,
};
use crate::data::val::unit::ListPageUnitInfosVal;
use crate::model::shared::user::UserToken;
use crate::part::nucl::RepeatableRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;

/// `GET /api/v1/pages/{page_id}/units` — list units under a page.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/pages/{page_id}/units",
    tag = "units",
    params(("page_id" = String, Path, description = "Page ID")),
    responses(
        (status = 200, description = "Units listed", body = HttpBody<ListPageUnitInfosVal>),
        (status = 403, description = "No perm to list units in this page"),
        (status = 404, description = "Page not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ListPageUnitInfosVal> {
    //
    let instr = ListPageUnitInfosInstr { page_id };

    usecase::unit::list_infos::<RdbContext<RepeatableRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/units/save` — save Unit edits.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/units/save",
    tag = "units",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = Vec<UnitEditInstr>,
    responses(
        (status = 204, description = "Unit edits saved"),
        (status = 403, description = "No perm to save units in this page"),
        (status = 422, description = "Invalid Unit edit"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn save_infos(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(edits): Json<Vec<UnitEditInstr>>,
) -> HttpNoContent {
    //
    let instr = SavePageUnitEditsInstr { page_id, edits };

    usecase::unit::save_edits::<_, RdbContext<RepeatableRead>, HybRepo>(
        (harn.nucl_repeatable_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}
