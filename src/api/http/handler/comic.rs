//! Comic handlers: CRUD, cover upload flow, and completion toggle.

use axum::Json;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;

use serde::Deserialize;

use tracing::instrument;

use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;
use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpNoContent;
use crate::api::http::result::HttpResult;
use crate::api::http::result::NoContent;
use crate::api::http::state::AppHarn;
use crate::data::comic::{
    ComicInfoVal, CreateComicData, CreateComicVal, ListComicInfosData, MarkComicCompletedData,
    MarkComicCoverUploadedData, ReserveComicCoverData, ReserveComicCoverVal, UpdateComicInfoData,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::comic::{ComicInclOpt, ComicWithOpt};

/// Query for listing comics within a workset.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ComicListQuery {
    pub fuzzy_title: Option<String>,
    pub is_completed: Option<bool>,

    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<ComicInclOpt>,

    #[serde(default, rename = "with")]
    pub with_opt: Vec<ComicWithOpt>,

    pub offset: u64,
    pub limit: u64,
}

/// `POST /api/v1/comics` — create a comic (and its first chapter).
#[utoipa::path(
    post,
    path = "/api/v1/comics",
    tag = "comics",
    request_body = CreateComicData,
    responses(
        (status = 201, description = "Comic created", body = CreateComicVal),
        (status = 403, description = "No permission to create comics in this workset"),
        (status = 404, description = "Workset not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<CreateComicData>,
) -> HttpResult<CreateComicVal> {
    let reply = usecase::comic::create(harn.drive(), harn.repo(), user_token, data).await?;

    reply.accept(StatusCode::CREATED)
}

/// `GET /api/v1/worksets/{workset_id}/comics` — list comics in a workset.
#[utoipa::path(
    get,
    path = "/api/v1/worksets/{workset_id}/comics",
    tag = "comics",
    params(("workset_id" = String, Path, description = "Workset ID"), ComicListQuery),
    responses(
        (status = 200, description = "Comics listed", body = Vec<ComicInfoVal>),
        (status = 403, description = "No permission to list comics in this workset"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ComicListQuery>,
) -> HttpResult<Vec<ComicInfoVal>> {
    let data = ListComicInfosData {
        workset_id,
        fuzzy_title: query.fuzzy_title,
        is_completed: query.is_completed,
        incl_opt: query.incl_opt,
        with_opt: query.with_opt,
        offset: query.offset,
        limit: query.limit,
    };

    let infos =
        usecase::comic::list_infos(harn.repo(), harn.image_pool(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}

/// `GET /api/v1/comics/{comic_id}` — fetch a comic by id.
#[utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 200, description = "Comic info retrieved", body = ComicInfoVal),
        (status = 403, description = "No permission to view this comic"),
        (status = 404, description = "Comic not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ComicInfoVal> {
    let info =
        usecase::comic::get_info(harn.repo(), harn.image_pool(), user_token, comic_id).await?;

    info.accept(StatusCode::OK)
}

/// `PUT /api/v1/comics/{comic_id}` — update a comic's profile.
#[utoipa::path(
    put,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = UpdateComicInfoData,
    responses(
        (status = 204, description = "Comic updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this comic"),
        (status = 404, description = "Comic not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<UpdateComicInfoData>,
) -> HttpNoContent {
    ensure_path_matches_body_id(&comic_id, &data.id)?;

    usecase::comic::update_info(harn.repo(), user_token, data).await?;

    Ok(NoContent)
}

/// `POST /api/v1/comics/{comic_id}/cover/reserve` — reserve a cover upload slot.
#[utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/cover/reserve",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = ReserveComicCoverData,
    responses(
        (status = 200, description = "Cover upload URL reserved", body = ReserveComicCoverVal),
        (status = 403, description = "No permission to modify this comic's cover"),
        (status = 404, description = "Comic not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn reserve_cover(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<ReserveComicCoverData>,
) -> HttpResult<ReserveComicCoverVal> {
    let reply = usecase::comic::reserve_cover(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        comic_id,
        data,
    )
    .await?;

    reply.accept(StatusCode::OK)
}

/// `POST /api/v1/comics/{comic_id}/cover/mark-uploaded` — confirm a cover upload.
#[utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/cover/mark-uploaded",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = MarkComicCoverUploadedData,
    responses(
        (status = 204, description = "Cover upload confirmed"),
        (status = 403, description = "No permission to modify this comic's cover"),
        (status = 404, description = "Comic not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn mark_cover_uploaded(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<MarkComicCoverUploadedData>,
) -> HttpNoContent {
    usecase::comic::mark_cover_uploaded(harn.repo(), user_token, comic_id, data).await?;

    Ok(NoContent)
}

/// `POST /api/v1/comics/{comic_id}/mark-completed` — toggle comic completion.
#[utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/mark-completed",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = MarkComicCompletedData,
    responses(
        (status = 204, description = "Comic completion toggled"),
        (status = 403, description = "No permission to modify this comic"),
        (status = 404, description = "Comic not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn mark_completed(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<MarkComicCompletedData>,
) -> HttpNoContent {
    usecase::comic::mark_completed(
        harn.drive(),
        harn.repo(),
        user_token,
        comic_id,
        data.is_completed,
    )
    .await?;

    Ok(NoContent)
}

/// `DELETE /api/v1/comics/{comic_id}` — delete a comic and descendants.
#[utoipa::path(
    delete,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 204, description = "Comic deleted"),
        (status = 403, description = "No permission to delete this comic"),
        (status = 404, description = "Comic not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    usecase::comic::delete(harn.drive(), harn.repo(), harn.prom(), user_token, comic_id).await?;

    Ok(NoContent)
}
