//! Unit handlers: list and save page unit sequences.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::unit::{
    ListPageUnitInfosInstr, SavePageUnitEditsInstr,
    SearchChapterUnitInfosInstr, TransformChapterUnitsInstr, UnitEditInstr,
};
use crate::data::val::unit::ListPageUnitInfosVal;
use crate::data::view::unit::UnitInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::unit::UnitTextPart;

/// Query parameters for Chapter Unit text searches.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
#[serde(deny_unknown_fields)]
pub struct UnitSearchQuery {
    //
    /// Unit text field selected for matching.
    pub part: UnitTextPart,
    /// Case-sensitive literal substring to search.
    pub phrase: String,
}

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

    usecase::unit::list_infos::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/units/search` — search Unit text.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/units/search",
    tag = "units",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        UnitSearchQuery,
    ),
    responses(
        (status = 200, description = "Matching Units listed in page order", body = HttpBody<Vec<UnitInfoView>>),
        (status = 403, description = "No perm to search Units in this Chapter"),
        (status = 422, description = "Search phrase is empty or more than 100 Units match"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn search_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<UnitSearchQuery>,
) -> HttpResult<Vec<UnitInfoView>> {
    //
    let instr = SearchChapterUnitInfosInstr {
        chapter_id,
        part: query.part,
        phrase: query.phrase,
    };

    usecase::unit::search_infos::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
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
        (status = 409, description = "Serializable conflict; retry the complete request"),
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

    usecase::unit::save_edits::<_, RdbContext<Serial>, HybRepo>(
        (harn.nucl().serial(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/chapters/{chapter_id}/units/transform` — transform Unit text.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/units/transform",
    tag = "units",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = TransformChapterUnitsInstr,
    responses(
        (status = 204, description = "Unit transforms applied"),
        (status = 403, description = "Matching Chapter role required"),
        (status = 409, description = "Serializable conflict; retry the complete request"),
        (status = 422, description = "Invalid or overlapping Unit transform"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn transform(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<TransformChapterUnitsInstr>,
) -> HttpNoContent {
    //
    usecase::unit::transform::transform::<_, RdbContext<Serial>, HybRepo>(
        (harn.nucl().serial(), harn.repo()),
        user_token,
        chapter_id,
        instr,
    )
    .await?;

    no_content()
}
