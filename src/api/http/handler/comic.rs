//! Comic handlers: CRUD, cover upload flow, and immutable archiving.

use axum::Json;
use axum::extract::{Extension, Path, State};
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
use crate::data::comic::{
    ComicInfoVal, CreateComicParams, CreateComicPayload, ListComicInfosParams,
    MarkComicCoverUploadedParams, ReserveComicCoverParams,
    ReserveComicCoverPayload, UpdateComicInfoParams,
};
use crate::data::comic_archive::ArchiveComicPayload;
use crate::data::comic_list::ListComicInfosPayload;
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::comic::{ComicInclOpt, ComicWithOpt};
use crate::value::query::GroupedQuery;

/// Query for listing comics within a workset.
///
/// When present, `stages` narrows the list by pinned chapter workflow state.
///
/// `incl` embeds related rows into each item; `with` populates derived rows
/// in the parallel list payload.
/// Dotted `incl` values implicitly pull in their parent segments.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ComicListQuery {
    /// Fuzzy title substring filter (case-insensitive).
    pub fuzzy_title: Option<String>,

    /// Workflow stage bitmask filter for pinned chapters.
    pub stages: Option<u32>,

    /// Related rows to embed. Repeatable. Values: `workset`, `workset.team`,
    /// `creator`. Dotted values imply their parent segments.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<ComicInclOpt>,

    /// Derived rows to attach. Repeatable. Values: `pinned_chapter`,
    /// `pinned_chapter_assignment`. The assignment option requires the chapter
    /// option.
    #[serde(
        default,
        rename = "with",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub with_opt: Vec<ComicWithOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/comics` — create a comic (and its first chapter).
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/comics",
    tag = "comics",
    request_body = CreateComicParams,
    responses(
        (status = 201, description = "Comic created", body = HttpBody<CreateComicPayload>),
        (status = 403, description = "No permission to create comics in this workset"),
        (status = 404, description = "Workset not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<CreateComicParams>,
) -> HttpResult<CreateComicPayload> {
    usecase::comic::create(harn.drive(), harn.repo(), user_token, params)
        .await?
        .accept(StatusCode::CREATED)
}

/// `GET /api/v1/worksets/{workset_id}/comics` — list comics in a workset.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/worksets/{workset_id}/comics",
    tag = "comics",
    description = "Lists active comics in a workset with optional title and workflow-stage filters. `incl` embeds related rows and `with` populates parallel derived rows.",
    params(("workset_id" = String, Path, description = "Workset ID"), ComicListQuery),
    responses(
        (status = 200, description = "Comics listed", body = HttpBody<ListComicInfosPayload>),
        (status = 403, description = "No permission to list comics in this workset"),
        (status = 422, description = "Invalid query option combination or workflow-stage filter"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    GroupedQuery(query): GroupedQuery<ComicListQuery>,
) -> HttpResult<ListComicInfosPayload> {
    //
    let params = ListComicInfosParams {
        workset_id,
        fuzzy_title: query.fuzzy_title,
        stages: query.stages,
        incl_opt: query.incl_opt,
        with_opt: query.with_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::comic::list_infos(
        harn.repo(),
        harn.image_pool(),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/comics/{comic_id}` — fetch a comic by id.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 200, description = "Comic info retrieved", body = HttpBody<ComicInfoVal>),
        (status = 403, description = "No permission to view this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ComicInfoVal> {
    usecase::comic::get_info(
        harn.repo(),
        harn.image_pool(),
        user_token,
        comic_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/comics/{comic_id}` — update a comic's profile.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    put,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = UpdateComicInfoParams,
    responses(
        (status = 204, description = "Comic updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<UpdateComicInfoParams>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&comic_id, &params.id)?;

    usecase::comic::update_info(harn.repo(), user_token, params).await?;

    no_content()
}

/// `POST /api/v1/comics/{comic_id}/cover/reserve` — reserve a cover upload slot.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/cover/reserve",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = ReserveComicCoverParams,
    responses(
        (status = 200, description = "Cover upload URL reserved", body = HttpBody<ReserveComicCoverPayload>),
        (status = 403, description = "No permission to modify this comic's cover"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_cover(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<ReserveComicCoverParams>,
) -> HttpResult<ReserveComicCoverPayload> {
    usecase::comic::reserve_cover(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        comic_id,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/comics/{comic_id}/cover/mark-uploaded` — confirm a cover upload.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/cover/mark-uploaded",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = MarkComicCoverUploadedParams,
    responses(
        (status = 204, description = "Cover upload confirmed"),
        (status = 403, description = "No permission to modify this comic's cover"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn mark_cover_uploaded(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<MarkComicCoverUploadedParams>,
) -> HttpNoContent {
    //
    usecase::comic::mark_cover_uploaded(
        harn.repo(),
        user_token,
        comic_id,
        params,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/comics/{comic_id}/archive` — archive and remove one active comic.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/archive",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 201, description = "Comic archived", body = HttpBody<ArchiveComicPayload>),
        (status = 403, description = "No permission to archive this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn archive(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ArchiveComicPayload> {
    usecase::comic_archive::archive(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        user_token,
        comic_id,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `DELETE /api/v1/comics/{comic_id}` — delete a comic and descendants.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    delete,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 204, description = "Comic deleted"),
        (status = 403, description = "No permission to delete this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::comic::delete(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        user_token,
        comic_id,
    )
    .await?;

    no_content()
}
