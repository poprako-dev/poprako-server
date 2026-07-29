//! Terminology-entry handlers.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;

#[cfg(feature = "swagger-ui")]
use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;
#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::term::{
    CreateTermParams, CreateTermPayload, ListTermInfosParams, TermInfoVal,
    UpdateTermInfoParams,
};
use crate::model::user::UserToken;
use crate::usecase;

/// Query parameters for terms inside one terminology base.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct TermListQuery {
    /// Optional case-insensitive source substring.
    pub fuzzy_source: Option<String>,

    pub offset: u32,
    pub limit: u32,
}

/// `POST /api/v1/terms` — create a terminology entry.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/terms",
    tag = "terms",
    request_body = CreateTermParams,
    responses(
        (status = 201, description = "Term created", body = HttpBody<CreateTermPayload>),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Invalid term, duplicate source, or missing termbase"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<CreateTermParams>,
) -> HttpResult<CreateTermPayload> {
    usecase::term::create(harn.drive(), harn.repo(), user_token, params)
        .await?
        .accept(StatusCode::CREATED)
}

/// `GET /api/v1/termbases/{termbase_id}/terms` — list terms in one base.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}/terms",
    tag = "terms",
    params(("termbase_id" = String, Path, description = "Termbase ID"), TermListQuery),
    responses(
        (status = 200, description = "Terms listed", body = HttpBody<Vec<TermInfoVal>>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TermListQuery>,
) -> HttpResult<Vec<TermInfoVal>> {
    //
    let params = ListTermInfosParams {
        termbase_id,
        fuzzy_source: query.fuzzy_source,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::term::list_infos(harn.repo(), user_token, params)
        .await?
        .accept(StatusCode::OK)
}

/// `GET /api/v1/terms/{term_id}` — fetch a terminology entry.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    responses(
        (status = 200, description = "Term retrieved", body = HttpBody<TermInfoVal>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Term not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<TermInfoVal> {
    usecase::term::get_info(harn.repo(), user_token, term_id)
        .await?
        .accept(StatusCode::OK)
}

/// `PUT /api/v1/terms/{term_id}` — replace terminology-entry fields.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    put,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    request_body = UpdateTermInfoParams,
    responses(
        (status = 204, description = "Term updated"),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Invalid term, duplicate source, or path mismatch"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<UpdateTermInfoParams>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&term_id, &params.id)?;

    usecase::term::update_info(harn.drive(), harn.repo(), user_token, params)
        .await?;

    no_content()
}

/// `DELETE /api/v1/terms/{term_id}` — delete a terminology entry.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::term::delete(harn.drive(), harn.repo(), user_token, term_id)
        .await?;

    no_content()
}
