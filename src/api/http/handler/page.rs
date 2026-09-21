//! Page handlers: list, delete, batch allocation, and single image upload flow.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::page::{
    AllocChapterPagesInstr, AllocPageImageInstr, ListPageInfosInstr,
    ListPageUnitDiffStatsInstr, ListPageUnitFlaggedStatsInstr,
    MarkPageImageUploadedInstr,
};
use crate::data::val::page::{
    AllocChapterPagesVal, AllocatedPageVal, PageUnitDiffStatsVal,
    PageUnitFlaggedStatsVal,
};
use crate::data::view::page::PageInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::prom::rdb_impl::RdbProm;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase::page as page_usecase;
use crate::usecase::page::{
    alloc as page_alloc_usecase, delete as page_delete_usecase,
    list as page_list_usecase,
};

/// `GET /api/v1/chapters/{chapter_id}/pages` — list pages in a chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/pages",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Pages listed", body = HttpBody<Vec<PageInfoView>>),
        (status = 403, description = "No perm to list pages in this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Vec<PageInfoView>> {
    //
    let instr = ListPageInfosInstr { chapter_id };

    page_list_usecase::list_infos::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/pages/unit-diff-stats` — list Unit text statistics for Pages with revision differences.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/pages/unit-diff-stats",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Visible Unit text counts for Pages with edits or revision-only additions, in original Page order", body = HttpBody<Vec<PageUnitDiffStatsVal>>),
        (status = 403, description = "No perm to list Pages in this Chapter"),
        (status = 422, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_unit_diff_stats(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Vec<PageUnitDiffStatsVal>> {
    //
    let instr = ListPageUnitDiffStatsInstr { chapter_id };

    page_list_usecase::list_unit_diff_stats::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/pages/unit-flagged-stats` — list Pages containing visible flagged Units.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/pages/unit-flagged-stats",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Visible flagged Unit counts for Pages, in original Page order", body = HttpBody<Vec<PageUnitFlaggedStatsVal>>),
        (status = 403, description = "No perm to list Pages in this Chapter"),
        (status = 422, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_unit_flagged_stats(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Vec<PageUnitFlaggedStatsVal>> {
    //
    let instr = ListPageUnitFlaggedStatsInstr { chapter_id };

    page_list_usecase::list_unit_flagged_stats::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/pages/{page_id}` — fetch one page.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/pages/{page_id}",
    tag = "pages",
    params(("page_id" = String, Path, description = "Page ID")),
    responses(
        (status = 200, description = "Page info retrieved", body = HttpBody<PageInfoView>),
        (status = 403, description = "No perm to view this page"),
        (status = 404, description = "Page not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<PageInfoView> {
    //
    page_list_usecase::get_info::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        page_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `DELETE /api/v1/chapters/{chapter_id}/pages` — delete all pages in a chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/chapters/{chapter_id}/pages",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 204, description = "All pages deleted"),
        (status = 403, description = "No perm to delete pages in this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    page_delete_usecase::delete::<_, RdbContext<ReptRead>, HybRepo, _>(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        user_token,
        chapter_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/chapters/{chapter_id}/pages/alloc` — allocate all page images.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/pages/alloc",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = AllocChapterPagesInstr,
    responses(
        (status = 200, description = "Page upload slots allocated", body = HttpBody<AllocChapterPagesVal>),
        (status = 422, description = "Path id does not match body chapter id"),
        (status = 403, description = "No perm to allocate pages in this chapter"),
        (status = 422, description = "Invalid authoritative manifest, duplicate page identity, image metadata, or published chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_chapter_pages(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<AllocChapterPagesInstr>,
) -> HttpResult<AllocChapterPagesVal> {
    //
    ensure_path_matches_body_id(&chapter_id, &instr.chapter_id)?;

    page_alloc_usecase::alloc_chapter_pages::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
        RdbProm,
        _,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.prom(),
            harn.obj_dept(),
            &harn.config().image,
        ),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/image/alloc` — allocate a replacement page image.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/image/alloc",
    tag = "pages",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = AllocPageImageInstr,
    responses(
        (status = 200, description = "Page image upload URL allocated", body = HttpBody<AllocatedPageVal>),
        (status = 403, description = "No perm to modify this page's image"),
        (status = 422, description = "Page not found, conflicting image metadata, or published chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_image(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<AllocPageImageInstr>,
) -> HttpResult<AllocatedPageVal> {
    //
    page_alloc_usecase::alloc_image::<_, RdbContext<ReptRead>, HybRepo, _, _>(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.prom(),
            harn.obj_dept(),
            &harn.config().image,
        ),
        user_token,
        page_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/image/mark-uploaded` — confirm a page image upload.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/image/mark-uploaded",
    tag = "pages",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = MarkPageImageUploadedInstr,
    responses(
        (status = 204, description = "Page image upload confirmed"),
        (status = 403, description = "No perm to modify this page's image"),
        (status = 422, description = "Page, image identity, storage object, or published chapter is invalid"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_image_uploaded(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkPageImageUploadedInstr>,
) -> HttpNoContent {
    //
    page_usecase::mark_image_uploaded::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        page_id,
        instr,
    )
    .await?;

    no_content()
}
