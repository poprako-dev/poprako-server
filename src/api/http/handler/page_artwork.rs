//! Independent composite page HTTP delivery.
use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::page_artwork::{
    AllocChapterPageArtworksBody, AllocChapterPageArtworksPayload,
    AllocPageArtworkImageBody, AllocatedPageArtworkPayload,
    MarkPageArtworkImageUploadedBody, PageArtworkInfoObj,
};
use crate::api::http::result::{
    AcceptInto as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::page_artwork as page_artwork_usecase;
use crate::usecase::page_artwork::{
    image as page_artwork_image_usecase,
    manifest as page_artwork_manifest_usecase,
};

/// Lists the Chapter's independently ordered composite pages.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/page-artworks",
    tag = "page-artworks",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Composite pages",
            body = HttpBody<Vec<PageArtworkInfoObj>>),
        (status = 403, description = "Chapter read access required"),
        (status = 422, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(token): Extension<UserToken>,
) -> HttpResult<Vec<PageArtworkInfoObj>> {
    //
    Ok(
        page_artwork_usecase::list_infos::<RdbContext<ReptRead>, RdbRepo, _>(
            (harn.repo(), harn.obj_dept()),
            token,
            chapter_id,
        )
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// Replaces the authoritative composite manifest and allocates needed uploads.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/chapters/{chapter_id}/page-artworks/alloc",
        tag = "page-artworks",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
        request_body = AllocChapterPageArtworksBody,
    responses((status = 200, description = "Composite allocations",
        body = HttpBody<AllocChapterPageArtworksPayload>),
        (status = 403, description = "Composite write access required"),
            (status = 422, description = "Invalid manifest or published \
            Chapter"))
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_chapter_page_artworks(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(token): Extension<UserToken>,
    Json(instr): Json<AllocChapterPageArtworksBody>,
) -> HttpResult<AllocChapterPageArtworksPayload> {
    //
    Ok(
        page_artwork_manifest_usecase::alloc_chapter_page_artworks::<
            _,
            RdbContext<ReptRead>,
            RdbRepo,
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
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// Allocates a replacement composite image while preserving issues.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/page-artworks/{page_artwork_id}/image/alloc",
        tag = "page-artworks",
    params(("page_artwork_id" = String, Path,
        description = "Composite \
        page ID")), request_body = AllocPageArtworkImageBody,
    responses((status = 200, description = "Image \
    allocation", body = HttpBody<AllocatedPageArtworkPayload>),
        (status = 403, description = "Composite write access required"),
            (status = 422, description = "Invalid input, missing page or \
            published Chapter"))
))]
#[instrument(level = "info", skip_all)]
pub async fn alloc_image(
    State(harn): State<AppHarn>,
    Path(page_artwork_id): Path<String>,
    Extension(token): Extension<UserToken>,
    Json(instr): Json<AllocPageArtworkImageBody>,
) -> HttpResult<AllocatedPageArtworkPayload> {
    //
    Ok(page_artwork_image_usecase::alloc_image::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.obj_dept(),
            &harn.config().image,
        ),
        token,
        page_artwork_id,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::OK))
}

/// Marks an exact composite generation uploaded without changing workflow.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/page-artworks/{page_artwork_id}/image/mark-uploaded",
    tag = "page-artworks",
    params(("page_artwork_id" = String, Path,
        description = "Composite page ID")),
    request_body = MarkPageArtworkImageUploadedBody,
    responses(
        (status = 204, description = "Generation confirmed"),
        (status = 403, description = "Composite write access required"),
        (status = 422, description = "Stale generation, missing page or \
            published Chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_image_uploaded(
    State(harn): State<AppHarn>,
    Path(page_artwork_id): Path<String>,
    Extension(token): Extension<UserToken>,
    Json(instr): Json<MarkPageArtworkImageUploadedBody>,
) -> HttpNoContent {
    //
    page_artwork_image_usecase::mark_image_uploaded::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
    >(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        token,
        page_artwork_id,
        instr.into(),
    )
    .await?;

    Ok(no_content())
}
