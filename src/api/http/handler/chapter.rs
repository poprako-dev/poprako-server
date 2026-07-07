//! Chapter handlers: CRUD, pinned chapter, and workflow stage advance.

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
use crate::data::chapter::{
    ChapterInfoVal, CreateChapterData, CreateChapterVal, ListChapterInfosData,
    PatchChapterInfoData, UpdateChapterStageData,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::chapter::ChapterInclOpt;

/// Query for listing chapters within a comic.
///
/// `incl` embeds related rows into each item; dotted values implicitly pull
/// in their parent segments.
///
/// Example: `?incl=comic.workset.team&incl=creator&offset=0&limit=20`.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ChapterListQuery {
    /// Related rows to embed. Repeatable. Values: `comic`, `comic.workset`,
    /// `comic.workset.team`, `comic.creator`, `creator`. Dotted values imply
    /// their parent segments.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<ChapterInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u64,

    /// Maximum number of items to return.
    pub limit: u64,
}

/// `POST /api/v1/chapters` — create a chapter under a comic.
#[utoipa::path(
    post,
    path = "/api/v1/chapters",
    tag = "chapters",
    request_body = CreateChapterData,
    responses(
        (status = 201, description = "Chapter created", body = HttpBody<CreateChapterVal>),
        (status = 403, description = "No permission to create chapters in this comic"),
        (status = 404, description = "Comic not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<CreateChapterData>,
) -> HttpResult<CreateChapterVal> {
    let reply = usecase::chapter::create(harn.drive(), harn.repo(), user_token, data).await?;
    reply.accept(StatusCode::CREATED)
}

/// `GET /api/v1/comics/{comic_id}/chapters` — list chapters in a comic.
#[utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}/chapters",
    tag = "chapters",
    description = "Lists chapters of a comic. `incl` embeds related rows; dotted values imply their parent segments. Example: `/api/v1/comics/{comic_id}/chapters?incl=comic.workset.team&incl=creator&offset=0&limit=20`.",
    params(("comic_id" = String, Path, description = "Comic ID"), ChapterListQuery),
    responses(
        (status = 200, description = "Chapters listed", body = HttpBody<Vec<ChapterInfoVal>>),
        (status = 403, description = "No permission to list chapters in this comic"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ChapterListQuery>,
) -> HttpResult<Vec<ChapterInfoVal>> {
    let data = ListChapterInfosData {
        comic_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    let infos =
        usecase::chapter::list_infos(harn.repo(), harn.image_pool(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}

/// `GET /api/v1/comics/{comic_id}/chapters/pinned` — fetch the pinned chapter.
#[utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}/chapters/pinned",
    tag = "chapters",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 200, description = "Pinned chapter (or null)", body = HttpBody<Option<ChapterInfoVal>>),
        (status = 403, description = "No permission to view this comic's pinned chapter"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn get_pinned(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Option<ChapterInfoVal>> {
    let pinned = usecase::chapter::get_pinned(harn.repo(), user_token, comic_id).await?;
    pinned.accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}` — fetch a chapter by id.
#[utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Chapter info retrieved", body = HttpBody<ChapterInfoVal>),
        (status = 403, description = "No permission to view this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ChapterInfoVal> {
    let info = usecase::chapter::get_info(harn.repo(), user_token, chapter_id).await?;
    info.accept(StatusCode::OK)
}

/// `PATCH /api/v1/chapters/{chapter_id}` — partially update a chapter's profile.
#[utoipa::path(
    patch,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = PatchChapterInfoData,
    responses(
        (status = 204, description = "Chapter updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<PatchChapterInfoData>,
) -> HttpNoContent {
    ensure_path_matches_body_id(&chapter_id, &data.id)?;

    usecase::chapter::update_info(harn.drive(), harn.repo(), user_token, data).await?;

    no_content()
}

/// `POST /api/v1/chapters/{chapter_id}/stage/advance` — advance a workflow stage.
#[utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/stage/advance",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = UpdateChapterStageData,
    responses(
        (status = 204, description = "Stage advanced"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this chapter's stage"),
        (status = 400, description = "Illegal workflow transition"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn advance_stage(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<UpdateChapterStageData>,
) -> HttpNoContent {
    ensure_path_matches_body_id(&chapter_id, &data.id)?;

    usecase::chapter::update_stage(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.develop(),
        user_token,
        data,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/chapters/{chapter_id}` — delete a chapter and descendants.
#[utoipa::path(
    delete,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 204, description = "Chapter deleted"),
        (status = 403, description = "No permission to delete this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    usecase::chapter::delete(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        user_token,
        chapter_id,
    )
    .await?;
    no_content()
}
