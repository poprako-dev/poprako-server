//! Chapter translation port handlers: import and file-download export.

use axum::Json;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::CONTENT_DISPOSITION;
use axum::http::header::CONTENT_TYPE;
use axum::response::Response;

use serde::Deserialize;

use tracing::instrument;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpError;
use crate::api::http::result::HttpResult;
use crate::api::http::state::AppHarn;
use crate::data::chapter_port::{ChapterTranslationImportData, ChapterTranslationImportVal};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::chapter_port::TranslationFormat;

/// Query selecting the export format.
#[derive(Debug, Deserialize)]
pub struct TranslationExportQuery {
    pub format: TranslationFormat,
}

/// `POST /api/v1/chapters/{chapter_id}/translations/import` — import translations.
#[utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/translations/import",
    tag = "chapter-port",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = ChapterTranslationImportData,
    responses(
        (status = 200, description = "Translations imported", body = ChapterTranslationImportVal),
        (status = 403, description = "No permission to import into this chapter"),
        (status = 400, description = "Invalid import content for the selected format"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn import(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<ChapterTranslationImportData>,
) -> HttpResult<ChapterTranslationImportVal> {
    let reply =
        usecase::chapter_port::import(harn.drive(), harn.repo(), user_token, data, chapter_id)
            .await?;

    reply.accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/translations/export` — export as file download.
///
/// `format=poprako` returns a JSON document (`application/json`); `format=label-plus`
/// returns a LabelPlus text document (`text/plain`). Both use
/// `Content-Disposition: attachment` with a generated filename.
#[utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/translations/export",
    tag = "chapter-port",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("format" = TranslationFormat, Query, description = "Export format: poprako or label-plus"),
    ),
    responses(
        (status = 200, description = "Translation file download", content_type = "application/json"),
        (status = 403, description = "No permission to export this chapter"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn export(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TranslationExportQuery>,
) -> Result<Response, HttpError> {
    let filename = format!("chapter_{}", chapter_id);

    match query.format {
        TranslationFormat::PopRaKo => {
            let val = usecase::chapter_port::export(
                harn.repo(),
                harn.image_pool(),
                user_token,
                chapter_id,
            )
            .await?;

            let body = serde_json::to_vec(&val).map_err(|err| {
                tracing::warn!("[chapter_port::export] serialization failed: {}", err);

                HttpError::internal()
            })?;

            file_response(
                "application/json",
                &format!("{}.json", filename),
                Bytes::from(body),
            )
        }

        TranslationFormat::LabelPlus => {
            let content =
                usecase::chapter_port::export_label_plus(harn.repo(), user_token, chapter_id)
                    .await?;

            file_response(
                "text/plain; charset=utf-8",
                &format!("{}.txt", filename),
                Bytes::from(content),
            )
        }
    }
}

/// Builds a `200 OK` file-download response with the given content type and
/// `Content-Disposition: attachment; filename="<filename>"`.
fn file_response(content_type: &str, filename: &str, body: Bytes) -> Result<Response, HttpError> {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(body))
        .map_err(|err| {
            tracing::warn!("[chapter_port::file_response] build failed: {}", err);

            HttpError::internal()
        })
}
