//! Chapter handlers: CRUD, pinned chapter, and workflow stage advance.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
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
use crate::data::instr::chapter::{
    CreateChapterInstr, ListChapterInfosInstr,
    ListChapterWorkflowRecordInfosInstr, UpdateChapterInfoInstr,
    UpdateChapterStageInstr,
};
use crate::data::val::chapter::CreateChapterVal;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::chapter_workflow_record::ChapterWorkflowRecordInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::chapter::ChapterInclOpt;
use crate::value::pagination::PubListLimit;

/// Query for listing chapters within a comic.
///
/// `incl` embeds related rows into each item; dotted values implicitly pull
/// in their parent segments.
///
/// Example: `?incl=comic.workset.team&incl=creator&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ChapterListQuery {
    //
    /// Related rows to embed. Repeatable. Values: `comic`, `comic.workset`,
    /// `comic.workset.team`, `comic.creator`, `creator`. Dotted values imply
    /// their parent segments.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<ChapterInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: PubListLimit,
}

/// Query for listing immutable chapter workflow records.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ChapterWorkflowRecordListQuery {
    //
    /// Pagination offset (0-based).
    pub offset: u32,
    /// Maximum number of items to return.
    pub limit: PubListLimit,
}

/// `POST /api/v1/chapters` — create a chapter under a comic.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters",
    tag = "chapters",
    request_body = CreateChapterInstr,
    responses(
        (status = 201, description = "Chapter created", body = HttpBody<CreateChapterVal>),
        (status = 403, description = "No perm to create chapters in this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateChapterInstr>,
) -> HttpResult<CreateChapterVal> {
    //
    usecase::chapter::create::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/comics/{comic_id}/chapters` — list chapters in a comic.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}/chapters",
    tag = "chapters",
    description = "Lists chapters of a comic. `incl` embeds related rows; dotted values imply their parent segments. Example: `/api/v1/comics/{comic_id}/chapters?incl=comic.workset.team&incl=creator&offset=0&limit=20`.",
    params(("comic_id" = String, Path, description = "Comic ID"), ChapterListQuery),
    responses(
        (status = 200, description = "Chapters listed", body = HttpBody<Vec<ChapterInfoView>>),
        (status = 403, description = "No perm to list chapters in this comic"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ChapterListQuery>,
) -> HttpResult<Vec<ChapterInfoView>> {
    //
    let instr = ListChapterInfosInstr {
        comic_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::chapter::list_infos::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/comics/{comic_id}/chapters/pinned` — fetch the pinned chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}/chapters/pinned",
    tag = "chapters",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 200, description = "Pinned chapter (or null)", body = HttpBody<Option<ChapterInfoView>>),
        (status = 403, description = "No perm to view this comic's pinned chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_pinned(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Option<ChapterInfoView>> {
    //
    usecase::chapter::get_pinned::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        comic_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}` — fetch a chapter by id.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Chapter info retrieved", body = HttpBody<ChapterInfoView>),
        (status = 403, description = "No perm to view this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ChapterInfoView> {
    //
    usecase::chapter::get_info::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        chapter_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/workflow-records` — list activity records.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/workflow-records",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID"), ChapterWorkflowRecordListQuery),
    responses(
        (status = 200, description = "Chapter workflow records listed", body = HttpBody<Vec<ChapterWorkflowRecordInfoView>>),
        (status = 403, description = "No perm to view this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_workflow_record_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ChapterWorkflowRecordListQuery>,
) -> HttpResult<Vec<ChapterWorkflowRecordInfoView>> {
    //
    let instr = ListChapterWorkflowRecordInfosInstr {
        chapter_id,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::chapter::workflow_record::list_workflow_record_infos::<
        RdbContext<ReptRead>,
        HybRepo,
    >((harn.repo(),), user_token, instr)
    .await?
    .accept(StatusCode::OK)
}

/// `PATCH /api/v1/chapters/{chapter_id}` — partially update a chapter's profile.
#[cfg_attr(feature = "swagger", utoipa::path(
    patch,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = UpdateChapterInfoInstr,
    responses(
        (status = 204, description = "Chapter updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No perm to update this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateChapterInfoInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&chapter_id, &instr.id)?;

    usecase::chapter::update_info::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/chapters/{chapter_id}/mark-pinned` — pin a chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/mark-pinned",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 204, description = "Chapter marked as pinned"),
        (status = 403, description = "No perm to pin this chapter"),
        (status = 404, description = "Chapter not found"),
        (status = 422, description = "Published chapters cannot be pinned"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_pinned(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::chapter::mark_pinned::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        chapter_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/chapters/{chapter_id}/stage/advance` — advance a workflow stage.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/stage/advance",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = UpdateChapterStageInstr,
    responses(
        (status = 204, description = "Stage advanced"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No perm to update this chapter's stage"),
        (status = 422, description = "Illegal workflow transition"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn advance_stage(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateChapterStageInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&chapter_id, &instr.id)?;

    usecase::chapter::stage::update_stage::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
        _,
        _,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            harn.develop(),
        ),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/chapters/{chapter_id}` — delete a chapter and descendants.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 204, description = "Chapter deleted"),
        (status = 403, description = "No perm to delete this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::chapter::delete::delete::<_, RdbContext<Serial>, HybRepo, _>(
        (harn.nucl().serial(), harn.repo(), harn.obj_dept()),
        user_token,
        chapter_id,
    )
    .await?;

    no_content()
}
