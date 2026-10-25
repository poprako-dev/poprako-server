//! source page handlers: list, delete, batch allocation, and single image upload flow.

use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::page_source::{
    AllocChapterPageSourcesInstr, AllocPageSourceImageInstr,
    ListPageSourceInfosInstr, ListPageSourceUnitDiffStatsInstr,
    ListPageSourceUnitFlaggedStatsInstr, MarkPageSourceImageUploadedInstr,
};
use crate::data::val::page_source::{
    AllocChapterPageSourcesVal, AllocatedPageSourceVal,
    PageSourceUnitDiffStatsVal, PageSourceUnitFlaggedStatsVal,
};
use crate::data::view::page_source::PageSourceInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::prom::rdb_impl::writer::RdbProm;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::page_source as page_source_usecase;
use crate::usecase::page_source::{
    alloc as page_source_alloc_usecase, delete as page_source_delete_usecase,
    list as page_source_list_usecase,
};

/// `GET /api/v1/chapters/{chapter_id}/page-sources` — list pages in a chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/page-sources",
    tag = "page-sources",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "PageSources \
        listed", body = HttpBody<Vec<PageSourceInfoView>>),
        (status = 403, description = "No perm to list pages in this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Vec<PageSourceInfoView>> {
    //
    let instr = ListPageSourceInfosInstr { chapter_id };

    Ok(
        page_source_list_usecase::list_infos::<RdbContext<ReptRead>, RdbRepo, _>(
            (harn.repo(), harn.obj_dept()),
            user_token,
            instr,
        )
        .await?
        .accept(StatusCode::OK),
    )
}

/// `GET /api/v1/chapters/{chapter_id}/page-sources/unit-diff-stats` — list Unit text statistics for source pages with revision differences.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/page-sources/unit-diff-stats",
    tag = "page-sources",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Visible Unit text counts for \
        PageSources with edits or revision-only additions, \
        in original PageSource order",
                body = HttpBody<Vec<PageSourceUnitDiffStatsVal>>),
        (status = 403,
            description = "No perm to list PageSources in this Chapter"),
        (status = 422, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_unit_diff_stats(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Vec<PageSourceUnitDiffStatsVal>> {
    //
    let instr = ListPageSourceUnitDiffStatsInstr { chapter_id };

    Ok(page_source_list_usecase::list_unit_diff_stats::<
        RdbContext<ReptRead>,
        RdbRepo,
    >((harn.repo(),), user_token, instr)
    .await?
    .accept(StatusCode::OK))
}

/// `GET /api/v1/chapters/{chapter_id}/page-sources/unit-flagged-stats` — list source pages containing visible flagged Units.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/page-sources/unit-flagged-stats",
    tag = "page-sources",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Visible \
        flagged Unit counts for PageSources, in original PageSource order",
            body = HttpBody<Vec<PageSourceUnitFlaggedStatsVal>>),
        (status = 403,
            description = "No perm to list PageSources in this Chapter"),
        (status = 422, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_unit_flagged_stats(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Vec<PageSourceUnitFlaggedStatsVal>> {
    //
    let instr = ListPageSourceUnitFlaggedStatsInstr { chapter_id };

    Ok(page_source_list_usecase::list_unit_flagged_stats::<
        RdbContext<ReptRead>,
        RdbRepo,
    >((harn.repo(),), user_token, instr)
    .await?
    .accept(StatusCode::OK))
}

/// `GET /api/v1/page-sources/{page_source_id}` — fetch one page.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/page-sources/{page_source_id}",
    tag = "page-sources",
    params(("page_source_id" = String, Path, description = "PageSource ID")),
    responses(
        (status = 200, description = "PageSource \
        info retrieved", body = HttpBody<PageSourceInfoView>),
        (status = 403, description = "No perm to view this page"),
        (status = 404, description = "PageSource not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(page_source_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<PageSourceInfoView> {
    //
    Ok(
        page_source_list_usecase::get_info::<RdbContext<ReptRead>, RdbRepo, _>(
            (harn.repo(), harn.obj_dept()),
            user_token,
            page_source_id,
        )
        .await?
        .accept(StatusCode::OK),
    )
}

/// `DELETE /api/v1/chapters/{chapter_id}/page-sources` — delete all pages in a chapter.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/chapters/{chapter_id}/page-sources",
    tag = "page-sources",
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
    page_source_delete_usecase::delete::<_, RdbContext<ReptRead>, RdbRepo, _>(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        user_token,
        chapter_id,
    )
    .await?;

    Ok(no_content())
}

/// `POST /api/v1/chapters/{chapter_id}/page-sources/alloc` — allocate all page images.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/page-sources/alloc",
    tag = "page-sources",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = AllocChapterPageSourcesInstr,
    responses(
        (status = 200, description = "PageSource \
        upload slots allocated", body = HttpBody<AllocChapterPageSourcesVal>),
        (status = 422, description = "Path id does not match body chapter id"),
        (status = 403, description = "No perm to allocate pages in this \
        chapter"),
        (status = 422, description = "Invalid authoritative manifest, \
        duplicate page identity, image metadata, or published chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_chapter_page_sources(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<AllocChapterPageSourcesInstr>,
) -> HttpResult<AllocChapterPageSourcesVal> {
    //
    ensure_path_matches_body_id(&chapter_id, &instr.chapter_id)?;

    Ok(page_source_alloc_usecase::alloc_chapter_page_sources::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
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
    .accept(StatusCode::OK))
}

/// `POST /api/v1/page-sources/{page_source_id}/image/alloc` — allocate a replacement page image.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/page-sources/{page_source_id}/image/alloc",
    tag = "page-sources",
    params(("page_source_id" = String, Path, description = "PageSource ID")),
    request_body = AllocPageSourceImageInstr,
    responses(
        (status = 200, description = "PageSource \
        image upload URL allocated", body = HttpBody<AllocatedPageSourceVal>),
        (status = 403, description = "No perm to modify this page's image"),
        (status = 422, description = "PageSource not found, conflicting image \
        metadata, or published chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_image(
    State(harn): State<AppHarn>,
    Path(page_source_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<AllocPageSourceImageInstr>,
) -> HttpResult<AllocatedPageSourceVal> {
    //
    Ok(page_source_alloc_usecase::alloc_image::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
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
        page_source_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK))
}

/// `POST /api/v1/page-sources/{page_source_id}/image/mark-uploaded` — confirm a page image upload.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/page-sources/{page_source_id}/image/mark-uploaded",
    tag = "page-sources",
    params(("page_source_id" = String, Path, description = "PageSource ID")),
    request_body = MarkPageSourceImageUploadedInstr,
    responses(
        (status = 204, description = "PageSource image upload confirmed"),
        (status = 403, description = "No perm to modify this page's image"),
        (status = 422, description = "PageSource, image identity, \
        storage object, or published chapter is invalid"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_image_uploaded(
    State(harn): State<AppHarn>,
    Path(page_source_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkPageSourceImageUploadedInstr>,
) -> HttpNoContent {
    //
    page_source_usecase::mark_image_uploaded::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
    >(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        user_token,
        page_source_id,
        instr,
    )
    .await?;

    Ok(no_content())
}
