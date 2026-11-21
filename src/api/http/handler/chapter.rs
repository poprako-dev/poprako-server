//! Chapter handlers: CRUD, pinned chapter, and workflow stage advance.
use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::chapter::{
    ChapterInfoObj, ChapterListQuery, ChapterWorkflowRecordListQuery,
    CreateChapterBody, CreateChapterPayload, ListChapterInfosQuery,
    ListChapterWorkflowRecordInfosQuery, UpdateChapterInfoBody,
    UpdateChapterStageBody,
};
use crate::api::http::object::chapter_workflow_record::ChapterWorkflowRecordInfoObj;
use crate::api::http::result::{
    Accept as _, AcceptInto as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::chapter as chapter_usecase;
use crate::usecase::chapter::{
    delete as chapter_delete_usecase, stage as chapter_stage_usecase,
    workflow_record as chapter_workflow_record_usecase,
};

/// `POST /api/v1/chapters` — create a chapter under a comic.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters",
    tag = "chapters",
    request_body = CreateChapterBody,
    responses(
        (status = 201, description = "Chapter \
        created", body = HttpBody<CreateChapterPayload>),
        (status = 403, description = "No perm to create chapters in this \
        comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateChapterBody>,
) -> HttpResult<CreateChapterPayload> {
    //
    Ok(chapter_usecase::create::<_, RdbContext<ReptRead>, RdbRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::CREATED))
}

/// `GET /api/v1/comics/{comic_id}/chapters` — list chapters in a comic.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}/chapters",
    tag = "chapters",
    description = "Lists chapters of a comic. `incl` embeds related rows; \
    dotted values imply their parent segments. Example: \
    `/api/v1/comics/{comic_id}/chapters?incl=comic.workset.team&\
    incl=creator&offset=0&limit=20`.",
    params(("comic_id" = String, Path, description = "Comic ID"),
        ChapterListQuery),
    responses(
        (status = 200, description = "Chapters listed",
            body = HttpBody<Vec<ChapterInfoObj>>),
        (status = 403, description = "No perm to list chapters in this comic"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ChapterListQuery>,
) -> HttpResult<Vec<ChapterInfoObj>> {
    //
    let instr = ListChapterInfosQuery {
        comic_id,
        incl: query.incl,
        offset: query.offset,
        limit: query.limit,
    };

    Ok(
        chapter_usecase::list_infos::<RdbContext<ReptRead>, RdbRepo, _>(
            (harn.repo(), harn.obj_dept()),
            user_token,
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// `GET /api/v1/comics/{comic_id}/chapters/pinned` — fetch the pinned chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}/chapters/pinned",
    tag = "chapters",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 200, description = "Pinned \
        chapter (or null)", body = HttpBody<Option<ChapterInfoObj>>),
        (status = 403, description = "No perm to view this comic's pinned \
        chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_pinned(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Option<ChapterInfoObj>> {
    //
    let pinned_chapter = chapter_usecase::get_pinned::<
        RdbContext<ReptRead>,
        RdbRepo,
    >((harn.repo(),), user_token, comic_id)
    .await?;

    let pinned_chapter = pinned_chapter.map(Into::into);

    Ok(pinned_chapter.accept(StatusCode::OK))
}

/// `GET /api/v1/chapters/{chapter_id}` — fetch a chapter by id.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Chapter \
        info retrieved", body = HttpBody<ChapterInfoObj>),
        (status = 403, description = "No perm to view this chapter"),
        (status = 404, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ChapterInfoObj> {
    //
    Ok(chapter_usecase::get_info::<RdbContext<ReptRead>, RdbRepo>(
        (harn.repo(),),
        user_token,
        chapter_id,
    )
    .await?
    .accept_into(StatusCode::OK))
}

/// `GET /api/v1/chapters/{chapter_id}/workflow-records` — list activity records.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/workflow-records",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID"),
        ChapterWorkflowRecordListQuery),
    responses(
        (status = 200, description = "Chapter \
        workflow records listed",
            body = HttpBody<Vec<ChapterWorkflowRecordInfoObj>>),
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
) -> HttpResult<Vec<ChapterWorkflowRecordInfoObj>> {
    //
    let instr = ListChapterWorkflowRecordInfosQuery {
        chapter_id,
        offset: query.offset,
        limit: query.limit,
    };

    Ok(
        chapter_workflow_record_usecase::list_workflow_record_infos::<
            RdbContext<ReptRead>,
            RdbRepo,
        >((harn.repo(),), user_token, instr.into())
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// `PATCH /api/v1/chapters/{chapter_id}` — partially update a chapter's profile.
#[cfg_attr(feature = "swagger", utoipa::path(
    patch,
    path = "/api/v1/chapters/{chapter_id}",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    description = "Requires owning-team administrator membership; no chapter \
    assignment is needed.",
    request_body = UpdateChapterInfoBody,
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
    Json(instr): Json<UpdateChapterInfoBody>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&chapter_id, &instr.id)?;

    chapter_usecase::update_info::<_, RdbContext<ReptRead>, RdbRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr.into(),
    )
    .await?;

    Ok(no_content())
}

/// `POST /api/v1/chapters/{chapter_id}/mark-pinned` — pin a chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/mark-pinned",
    description = "Requires owning-team administrator membership; no chapter \
    assignment is needed.",
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
    chapter_usecase::mark_pinned::<_, RdbContext<ReptRead>, RdbRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        chapter_id,
    )
    .await?;

    Ok(no_content())
}

/// `POST /api/v1/chapters/{chapter_id}/stage/advance` — advance a workflow stage.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/stage/advance",
    tag = "chapters",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    description = "Owning-team administrators may advance or revert without a \
    chapter assignment or worker role holder. Other callers require the \
    appropriate chapter worker role. State transitions and published freezing \
    still apply.",
    request_body = UpdateChapterStageBody,
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
    Json(instr): Json<UpdateChapterStageBody>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&chapter_id, &instr.id)?;

    chapter_stage_usecase::update_stage::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
        _,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            harn.bus(),
        ),
        user_token,
        instr.into(),
    )
    .await?;

    Ok(no_content())
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
    chapter_delete_usecase::delete::<_, RdbContext<Serial>, RdbRepo, _>(
        (harn.nucl().serial(), harn.repo(), harn.obj_dept()),
        user_token,
        chapter_id,
    )
    .await?;

    Ok(no_content())
}
