//! Page handlers: list, delete, batch reserve, and single image upload flow.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;
use crate::data::instr::page::{
    ListPageInfosInstr, MarkPageImageUploadedInstr, ReserveChapterPagesInstr,
    ReservePageImageInstr,
};
use crate::data::val::page::{ReserveChapterPagesVal, ReservedPageVal};
use crate::data::view::page::PageInfoView;

#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::usecase;

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

    usecase::page::list_infos(
        (harn.repo(), harn.image_pool()),
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
    usecase::page::get_info(
        (harn.repo(), harn.image_pool()),
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
    usecase::page::delete(
        (harn.nucl(), harn.repo(), harn.prom()),
        user_token,
        chapter_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/chapters/{chapter_id}/pages/reserve` — reserve all page images.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/pages/reserve",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = ReserveChapterPagesInstr,
    responses(
        (status = 200, description = "Page upload slots reserved", body = HttpBody<ReserveChapterPagesVal>),
        (status = 422, description = "Path id does not match body chapter id"),
        (status = 403, description = "No perm to reserve pages in this chapter"),
        (status = 422, description = "Invalid authoritative manifest, duplicate page identity, image metadata, or published chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn reserve_chapter_pages(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<ReserveChapterPagesInstr>,
) -> HttpResult<ReserveChapterPagesVal> {
    //
    ensure_path_matches_body_id(&chapter_id, &instr.chapter_id)?;

    usecase::page::reserve_chapter_pages(
        (harn.nucl(), harn.repo(), harn.prom(), harn.image_pool()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/image/reserve` — reserve a replacement page image.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/image/reserve",
    tag = "pages",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = ReservePageImageInstr,
    responses(
        (status = 200, description = "Page image upload URL reserved", body = HttpBody<ReservedPageVal>),
        (status = 403, description = "No perm to modify this page's image"),
        (status = 422, description = "Page not found, conflicting image metadata, or published chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn reserve_image(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<ReservePageImageInstr>,
) -> HttpResult<ReservedPageVal> {
    //
    usecase::page::reserve_image(
        (harn.nucl(), harn.repo(), harn.prom(), harn.image_pool()),
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
    usecase::page::mark_image_uploaded(
        (harn.nucl(), harn.repo(), harn.image_pool()),
        user_token,
        page_id,
        instr,
    )
    .await?;

    no_content()
}
