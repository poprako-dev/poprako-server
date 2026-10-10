//! Independent composite page HTTP delivery.

use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::page_artwork::{
    AllocChapterPageArtworksInstr, AllocPageArtworkImageInstr,
    MarkPageArtworkImageUploadedInstr,
};
use crate::data::val::page_artwork::{
    AllocChapterPageArtworksVal, AllocatedPageArtworkVal,
};
use crate::data::view::page_artwork::PageArtworkInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase::page_artwork as page_artwork_usecase;
use crate::usecase::page_artwork::{
    image as page_artwork_image_usecase,
    manifest as page_artwork_manifest_usecase,
};

/// Lists the Chapter's independently ordered composite pages.
#[cfg_attr(feature = "swagger", utoipa::path(
    get, path = "/api/v1/chapters/{chapter_id}/page-artworks", tag = "page-artworks",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses((status = 200, description = "Composite pages", body = HttpBody<Vec<PageArtworkInfoView>>),
        (status = 403, description = "Chapter read access required"), (status = 422, description = "Chapter not found"))
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(token): Extension<UserToken>,
) -> HttpResult<Vec<PageArtworkInfoView>> {
    //
    page_artwork_usecase::list_infos::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        token,
        chapter_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// Replaces the authoritative composite manifest and allocates needed uploads.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/chapters/{chapter_id}/page-artworks/alloc", tag = "page-artworks",
    params(("chapter_id" = String, Path, description = "Chapter ID")), request_body = AllocChapterPageArtworksInstr,
    responses((status = 200, description = "Composite allocations", body = HttpBody<AllocChapterPageArtworksVal>),
        (status = 403, description = "Composite write access required"), (status = 422, description = "Invalid manifest or published Chapter"))
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_chapter_page_artworks(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(token): Extension<UserToken>,
    Json(instr): Json<AllocChapterPageArtworksInstr>,
) -> HttpResult<AllocChapterPageArtworksVal> {
    //
    page_artwork_manifest_usecase::alloc_chapter_page_artworks::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
        _,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            &harn.config().image,
        ),
        token,
        chapter_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// Allocates a replacement composite image while preserving issues.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/page-artworks/{page_artwork_id}/image/alloc", tag = "page-artworks",
    params(("page_artwork_id" = String, Path, description = "Composite page ID")), request_body = AllocPageArtworkImageInstr,
    responses((status = 200, description = "Image allocation", body = HttpBody<AllocatedPageArtworkVal>),
        (status = 403, description = "Composite write access required"), (status = 422, description = "Invalid input, missing page or published Chapter"))
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_image(
    State(harn): State<AppHarn>,
    Path(page_artwork_id): Path<String>,
    Extension(token): Extension<UserToken>,
    Json(instr): Json<AllocPageArtworkImageInstr>,
) -> HttpResult<AllocatedPageArtworkVal> {
    //
    page_artwork_image_usecase::alloc_image::<_, RdbContext<ReptRead>, HybRepo, _>(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            &harn.config().image,
        ),
        token,
        page_artwork_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// Marks an exact composite generation uploaded without changing workflow.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/page-artworks/{page_artwork_id}/image/mark-uploaded", tag = "page-artworks",
    params(("page_artwork_id" = String, Path, description = "Composite page ID")), request_body = MarkPageArtworkImageUploadedInstr,
    responses((status = 204, description = "Generation confirmed"),
        (status = 403, description = "Composite write access required"), (status = 422, description = "Stale generation, missing page or published Chapter"))
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_image_uploaded(
    State(harn): State<AppHarn>,
    Path(page_artwork_id): Path<String>,
    Extension(token): Extension<UserToken>,
    Json(instr): Json<MarkPageArtworkImageUploadedInstr>,
) -> HttpNoContent {
    //
    page_artwork_image_usecase::mark_image_uploaded::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
        _,
    >(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        token,
        page_artwork_id,
        instr,
    )
    .await?;

    no_content()
}
