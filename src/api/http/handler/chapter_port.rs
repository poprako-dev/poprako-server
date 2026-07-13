//! Chapter translation port handlers: import, body export, and download export.

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;
use serde::Deserialize;
use tracing::instrument;

#[allow(unused_imports)]
use crate::api::http::result::{Accept as _, HttpBody, HttpError, HttpResult};
use crate::api::http::state::AppHarn;
#[allow(unused_imports)]
use crate::data::chapter_port::ExportChapterTranslationPayload;
use crate::data::chapter_port::{
    ImportChapterTranslationParams, ImportChapterTranslationPayload,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::chapter_port::TranslationFormat;

/// Query selecting the export format.
#[derive(Debug, Deserialize)]
pub struct TranslationExportQuery {
    /// Export format: `poprako` or `label-plus`.
    pub format: TranslationFormat,
}

/// `POST /api/v1/chapters/{chapter_id}/translations/import` — import translations.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/translations/import",
    tag = "chapter-port",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = ImportChapterTranslationParams,
    responses(
        (status = 200, description = "Translations imported", body = HttpBody<ImportChapterTranslationPayload>),
        (status = 403, description = "No permission to import into this chapter"),
        (status = 422, description = "Invalid import content for the selected format"),
    ),
))]
#[instrument(err, skip(harn, params))]
pub async fn import(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<ImportChapterTranslationParams>,
) -> HttpResult<ImportChapterTranslationPayload> {
    usecase::chapter_port::import(
        harn.drive(),
        harn.repo(),
        user_token,
        params,
        chapter_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/translations/export` — export response body.
///
/// `format=poprako` returns a JSON document (`application/json`); `format=label-plus`
/// returns a LabelPlus text document (`text/plain`).
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/translations/export",
    tag = "chapter-port",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("format" = TranslationFormat, Query, description = "Export format: poprako or label-plus"),
    ),
    responses(
        (status = 200, description = "PopRaKo translation export", body = HttpBody<ExportChapterTranslationPayload>, content_type = "application/json"),
        (status = 200, description = "LabelPlus translation export", content_type = "text/plain"),
        (status = 403, description = "No permission to export this chapter"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn export(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TranslationExportQuery>,
) -> Result<Response, HttpError> {
    //
    let payload =
        export_payload(&harn, user_token, chapter_id, query.format).await?;

    body_response(payload)
}

/// `GET /api/v1/chapters/{chapter_id}/translations/export/download` — export as file download.
///
/// `format=poprako` downloads a JSON document (`application/json`);
/// `format=label-plus` downloads a LabelPlus text document (`text/plain`).
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/translations/export/download",
    tag = "chapter-port",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("format" = TranslationFormat, Query, description = "Export format: poprako or label-plus"),
    ),
    responses(
        (status = 200, description = "Translation file download", content_type = "application/json"),
        (status = 403, description = "No permission to export this chapter"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn export_download(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TranslationExportQuery>,
) -> Result<Response, HttpError> {
    //
    let filename = format!("chapter_{}", chapter_id);

    let payload =
        export_payload(&harn, user_token, chapter_id, query.format).await?;

    download_response(&filename, payload)
}

/// Internal payload carrying the serialised export content and its metadata.
struct TranslationExportPayload {
    /// MIME type of the response body.
    content_type: &'static str,
    /// File extension for download filenames.
    extension: &'static str,
    /// Raw bytes of the serialised export.
    body: Bytes,
}

/// Loads the export content from the usecase and builds the payload for the
/// selected format.
async fn export_payload(
    harn: &AppHarn,
    user_token: UserToken,
    chapter_id: String,
    format: TranslationFormat,
) -> Result<TranslationExportPayload, HttpError> {
    match format {
        //
        TranslationFormat::PopRaKo => {
            //
            let val = usecase::chapter_port::export(
                harn.repo(),
                user_token,
                chapter_id,
            )
            .await?;

            let body = serde_json::to_vec(&val).map_err(|err| {
                //
                tracing::warn!(
                    error = %err,
                    "[chapter_port::export_payload] serialization failed",
                );

                HttpError::internal()
            })?;

            Ok(TranslationExportPayload {
                content_type: "application/json",
                extension: "json",
                body: Bytes::from(body),
            })
        }

        TranslationFormat::LabelPlus => {
            //
            let content = usecase::chapter_port::export_label_plus(
                harn.repo(),
                user_token,
                chapter_id,
            )
            .await?;

            Ok(TranslationExportPayload {
                content_type: "text/plain; charset=utf-8",
                extension: "txt",
                body: Bytes::from(content),
            })
        }
    }
}

/// Builds a `200 OK` export response with the given content type.
fn body_response(
    payload: TranslationExportPayload,
) -> Result<Response, HttpError> {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, payload.content_type)
        .body(Body::from(payload.body))
        .map_err(|err| {
            //
            tracing::warn!(
                error = %err,
                "[chapter_port::body_response] build failed",
            );

            HttpError::internal()
        })
}

/// Builds a `200 OK` file-download response with the given content type and
/// `Content-Disposition: attachment; filename="<filename>"`.
fn download_response(
    filename_base: &str,
    payload: TranslationExportPayload,
) -> Result<Response, HttpError> {
    //
    let filename = format!("{}.{}", filename_base, payload.extension);

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, payload.content_type)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(payload.body))
        .map_err(|err| {
            //
            tracing::warn!(
                error = %err,
                "[chapter_port::download_response] build failed",
            );

            HttpError::internal()
        })
}
