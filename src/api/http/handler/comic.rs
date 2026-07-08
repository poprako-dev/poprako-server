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
use crate::api::http::result::HttpBody;
use crate::api::http::result::HttpNoContent;
use crate::api::http::result::HttpResult;
use crate::api::http::result::no_content;
use crate::api::http::state::AppHarn;
use crate::data::comic::{
    ComicInfoVal, CreateComicData, CreateComicVal, ListComicInfosData, MarkComicCompletedData,
    MarkComicCoverUploadedData, ReserveComicCoverData, ReserveComicCoverVal, UpdateComicInfoData,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::comic::{ComicInclOpt, ComicWithOpt};

/// Query for listing comics within a workset.
///
/// Filtering modes are selected by `is_completed` and `stages` together:
/// - omit both: list all comics in the workset;
/// - `is_completed=true`: list completed comics only (`stages` must be
///   omitted — combining them is rejected with `422`);
/// - `is_completed=false`: list active comics, optionally narrowed by
///   `stages`;
/// - omit `is_completed` but pass `stages`: list active comics in those
///   stages.
///
/// `incl` embeds related rows into each item; `with` attaches derived rows.
/// Dotted `incl` values implicitly pull in their parent segments.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ComicListQuery {
    /// Fuzzy title substring filter (case-insensitive).
    pub fuzzy_title: Option<String>,

    /// Completion filter. `Some(true)` selects completed comics, `Some(false)`
    /// selects active comics, `None` leaves completion unconstrained. Must not
    /// be `Some(true)` together with `stages`.
    pub is_completed: Option<bool>,

    /// Workflow stage bitmask filter for active comics. Only meaningful when
    /// `is_completed` is not `Some(true)`; rejected otherwise.
    pub stages: Option<u32>,

    /// Related rows to embed. Repeatable. Values: `workset`, `workset.team`,
    /// `creator`. Dotted values imply their parent segments.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<ComicInclOpt>,

    /// Derived rows to attach. Repeatable. Values: `pinned_chapter`.
    #[serde(
        default,
        rename = "with",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub with_opt: Vec<ComicWithOpt>,

    /// FIXME: Paginate
    /// Pagination offset (0-based).
    pub offset: u64,

    /// Maximum number of items to return.
    pub limit: u64,
}

/// `POST /api/v1/comics` — create a comic (and its first chapter).
#[utoipa::path(
    post,
    path = "/api/v1/comics",
    tag = "comics",
    request_body = CreateComicData,
    responses(
        (status = 201, description = "Comic created", body = HttpBody<CreateComicVal>),
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
    usecase::comic::create(harn.drive(), harn.repo(), user_token, data)
        .await?
        .accept(StatusCode::CREATED)
}

/// `GET /api/v1/worksets/{workset_id}/comics` — list comics in a workset.
#[utoipa::path(
    get,
    path = "/api/v1/worksets/{workset_id}/comics",
    tag = "comics",
    description = "Lists comics in a workset with optional title, completion, and stage filters. `is_completed=true` must not be combined with `stages`; `is_completed=false` or omitting `is_completed` allows `stages`. `incl` embeds related rows, `with` attaches derived rows. Example: `/api/v1/worksets/{workset_id}/comics?is_completed=false&stages=6&incl=workset.team&incl=creator&with=pinned_chapter&offset=0&limit=20`.",
    params(("workset_id" = String, Path, description = "Workset ID"), ComicListQuery),
    responses(
        (status = 200, description = "Comics listed", body = HttpBody<Vec<ComicInfoVal>>),
        (status = 403, description = "No permission to list comics in this workset"),
        (status = 422, description = "Invalid argument combination (e.g. is_completed=true with stages)"),
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
        stages: query.stages,
        incl_opt: query.incl_opt,
        with_opt: query.with_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::comic::list_infos(harn.repo(), harn.image_pool(), user_token, data)
        .await?
        .accept(StatusCode::OK)
}

/// `GET /api/v1/comics/{comic_id}` — fetch a comic by id.
#[utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 200, description = "Comic info retrieved", body = HttpBody<ComicInfoVal>),
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
    usecase::comic::get_info(harn.repo(), harn.image_pool(), user_token, comic_id)
        .await?
        .accept(StatusCode::OK)
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

    no_content()
}

/// `POST /api/v1/comics/{comic_id}/cover/reserve` — reserve a cover upload slot.
#[utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/cover/reserve",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = ReserveComicCoverData,
    responses(
        (status = 200, description = "Cover upload URL reserved", body = HttpBody<ReserveComicCoverVal>),
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
    usecase::comic::reserve_cover(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        comic_id,
        data,
    )
    .await?
    .accept(StatusCode::OK)
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
    no_content()
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
    no_content()
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
    no_content()
}
