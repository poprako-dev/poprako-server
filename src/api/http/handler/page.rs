//! Page handlers: list, delete, batch reserve, and single image upload flow.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;

use tracing::instrument;

use crate::api::http::handler::util::{
    Pagination, ensure_path_matches_body_id,
};

#[cfg(feature = "swagger-ui")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::page_data;
use crate::model::user_model;
use crate::usecase;

/// `GET /api/v1/chapters/{chapter_id}/pages` — list pages in a chapter.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/pages",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID"), Pagination),
    responses(
        (status = 200, description = "Pages listed", body = HttpBody<Vec<page_data::InfoVal>>),
        (status = 403, description = "No permission to list pages in this chapter"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Query(pagination): Query<Pagination>,
) -> HttpResult<Vec<page_data::InfoVal>> {
    //
    let data = page_data::ListInfosData {
        chapter_id,
        offset: pagination.offset,
        limit: pagination.limit,
    };

    usecase::page::list_infos(harn.repo(), harn.image_pool(), user_token, data)
        .await?
        .accept(StatusCode::OK)
}

/// `DELETE /api/v1/chapters/{chapter_id}/pages` — delete all pages in a chapter.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    delete,
    path = "/api/v1/chapters/{chapter_id}/pages",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 204, description = "All pages deleted"),
        (status = 403, description = "No permission to delete pages in this chapter"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
) -> HttpNoContent {
    //
    usecase::page::delete(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        user_token,
        chapter_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/chapters/{chapter_id}/pages/reserve` — reserve all page images.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/pages/reserve",
    tag = "pages",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = page_data::ReserveChapterData,
    responses(
        (status = 200, description = "Page upload slots reserved", body = HttpBody<page_data::ReserveChapterVal>),
        (status = 422, description = "Path id does not match body chapter id"),
        (status = 403, description = "No permission to reserve pages in this chapter"),
        (status = 422, description = "Chapter already has pages or invalid page count"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn reserve_chapter_pages(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<page_data::ReserveChapterData>,
) -> HttpResult<page_data::ReserveChapterVal> {
    //
    ensure_path_matches_body_id(&chapter_id, &data.chapter_id)?;

    usecase::page::reserve_chapter_pages(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        data,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/image/reserve` — reserve a replacement page image.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/image/reserve",
    tag = "pages",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = page_data::ReserveImageData,
    responses(
        (status = 200, description = "Page image upload URL reserved", body = HttpBody<page_data::ReserveImageVal>),
        (status = 403, description = "No permission to modify this page's image"),
        (status = 404, description = "Page not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn reserve_image(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<page_data::ReserveImageData>,
) -> HttpResult<page_data::ReserveImageVal> {
    usecase::page::reserve_image(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        page_id,
        data,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/pages/{page_id}/image/mark-uploaded` — confirm a page image upload.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/pages/{page_id}/image/mark-uploaded",
    tag = "pages",
    params(("page_id" = String, Path, description = "Page ID")),
    request_body = page_data::MarkImageUploadedData,
    responses(
        (status = 204, description = "Page image upload confirmed"),
        (status = 403, description = "No permission to modify this page's image"),
        (status = 404, description = "Page not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn mark_image_uploaded(
    State(harn): State<AppHarn>,
    Path(page_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<page_data::MarkImageUploadedData>,
) -> HttpNoContent {
    //
    usecase::page::mark_image_uploaded(
        harn.drive(),
        harn.repo(),
        user_token,
        page_id,
        data,
    )
    .await?;

    no_content()
}
