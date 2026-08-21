//! Terminology-entry handlers.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::term::{
    CreateTermInstr, ListTermInfosInstr, UpdateTermInfoInstr,
};
use crate::data::val::term::CreateTermVal;
use crate::data::view::term::TermInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;

/// Query parameters for terms inside one terminology base.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct TermListQuery {
    //
    /// Optional case-insensitive source substring.
    pub fuzzy_source: Option<String>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}

/// `POST /api/v1/terms` — create a terminology entry.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/terms",
    tag = "terms",
    request_body = CreateTermInstr,
    responses(
        (status = 201, description = "Term created", body = HttpBody<CreateTermVal>),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Invalid term, duplicate source, or missing termbase"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateTermInstr>,
) -> HttpResult<CreateTermVal> {
    //
    usecase::term::create::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/termbases/{termbase_id}/terms` — list terms in one base.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}/terms",
    tag = "terms",
    params(("termbase_id" = String, Path, description = "Termbase ID"), TermListQuery),
    responses(
        (status = 200, description = "Terms listed", body = HttpBody<Vec<TermInfoView>>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TermListQuery>,
) -> HttpResult<Vec<TermInfoView>> {
    //
    let instr = ListTermInfosInstr {
        termbase_id,
        fuzzy_source: query.fuzzy_source,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::term::list_infos::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/terms/{term_id}` — fetch a terminology entry.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    responses(
        (status = 200, description = "Term retrieved", body = HttpBody<TermInfoView>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Term not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<TermInfoView> {
    //
    usecase::term::get_info::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        term_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/terms/{term_id}` — replace terminology-entry fields.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    request_body = UpdateTermInfoInstr,
    responses(
        (status = 204, description = "Term updated"),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Invalid term, duplicate source, or path mismatch"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateTermInfoInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&term_id, &instr.id)?;

    usecase::term::update_info::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/terms/{term_id}` — delete a terminology entry.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    responses(
        (status = 204, description = "Term deleted"),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Term not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::term::delete::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        term_id,
    )
    .await?;

    no_content()
}
