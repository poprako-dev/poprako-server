//! Unit handlers: list and save page unit sequences.
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::unit::{
    ListPageSourceUnitInfosBody, ListPageSourceUnitInfosPayload,
    SavePageSourceUnitEditsBody, SavePageSourceUnitEditsPayload,
    SearchChapterUnitInfosBody, TransformChapterUnitsBody, UnitEditBody,
    UnitInfoObj, UnitSaveQuery, UnitSearchQuery,
};
use crate::api::http::result::{
    AcceptInto as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::unit as unit_usecase;
use crate::usecase::unit::transform as unit_transform_usecase;

/// `GET /api/v1/page-sources/{page_source_id}/units` — list units under a page.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/page-sources/{page_source_id}/units",
    tag = "units",
    params(("page_source_id" = String, Path, description = "PageSource ID")),
    responses(
        (status = 200, description = "Units \
        listed", body = HttpBody<ListPageSourceUnitInfosPayload>),
        (status = 403, description = "No perm to list units in this page"),
        (status = 404, description = "PageSource not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(page_source_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ListPageSourceUnitInfosPayload> {
    //
    let instr = ListPageSourceUnitInfosBody { page_source_id };

    Ok(
        unit_usecase::list_infos::<_, RdbContext<ReptRead>, RdbRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::OK),
    )
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
        (status = 200, description = "Matching Units listed in page order",
            body = HttpBody<Vec<UnitInfoObj>>),
        (status = 403, description = "No perm to search Units in this Chapter"),
        (status = 422, description = "Search phrase is empty or more than \
        100 Units match"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn search_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<UnitSearchQuery>,
) -> HttpResult<Vec<UnitInfoObj>> {
    //
    let instr = SearchChapterUnitInfosBody {
        chapter_id,
        part: query.part,
        phrase: query.phrase,
    };

    Ok(
        unit_usecase::search_infos::<_, RdbContext<ReptRead>, RdbRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// `POST /api/v1/page-sources/{page_source_id}/units/save` — save Unit edits.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/page-sources/{page_source_id}/units/save",
    tag = "units",
    params(
        ("page_source_id" = String, Path, description = "PageSource ID"),
        UnitSaveQuery,
    ),
    request_body = Vec<UnitEditBody>,
    responses(
        (status = 200, description = "Unit \
        edits saved", body = HttpBody<SavePageSourceUnitEditsPayload>),
        (status = 409, description = "Serializable conflict; retry the \
        complete request"),
        (status = 403, description = "No perm to save units in this page"),
        (status = 422, description = "Invalid Unit edit"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn save_infos(
    State(harn): State<AppHarn>,
    Path(page_source_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<UnitSaveQuery>,
    Json(edits): Json<Vec<UnitEditBody>>,
) -> HttpResult<SavePageSourceUnitEditsPayload> {
    //
    let instr = SavePageSourceUnitEditsBody {
        page_source_id,
        edits,
        save_id: query.save_id,
    };

    Ok(
        unit_usecase::save_edits::<_, RdbContext<Serial>, RdbRepo, _>(
            (harn.nucl().serial(), harn.repo(), harn.cache()),
            user_token,
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// `POST /api/v1/chapters/{chapter_id}/units/transform` — transform Unit text.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/units/transform",
    tag = "units",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = TransformChapterUnitsBody,
    responses(
        (status = 204, description = "Unit transforms applied"),
        (status = 403, description = "Matching Chapter role required"),
        (status = 409, description = "Serializable conflict; retry the \
        complete request"),
        (status = 422, description = "Invalid or overlapping Unit transform"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn transform(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<TransformChapterUnitsBody>,
) -> HttpNoContent {
    //
    unit_transform_usecase::transform::<_, RdbContext<Serial>, RdbRepo>(
        (harn.nucl().serial(), harn.repo()),
        user_token,
        chapter_id,
        instr.into(),
    )
    .await?;

    Ok(no_content())
}
