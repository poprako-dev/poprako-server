//! Chapter translation port handlers: import, body export, and download export.

#[cfg(test)]
mod tests;

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;
#[cfg(feature = "swagger")]
use crate::data::val::chapter_port::ExportChapterTranslationsVal;

use crate::api::http::result::{Accept as _, HttpError, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::instr::chapter_port::{
    ExportChapterTranslationInstr, ImportChapterTranslationInstr,
};
use crate::data::val::chapter_port::ImportChapterTranslationVal;

use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::chapter_port::ExportFormatSpec;

/// `POST /api/v1/chapters/{chapter_id}/translations/import` — import translations.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/chapters/{chapter_id}/translations/import",
    tag = "chapter-port",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = ImportChapterTranslationInstr,
    responses(
        (status = 200, description = "Translations imported", body = HttpBody<ImportChapterTranslationVal>),
        (status = 403, description = "No perm to import into this chapter"),
        (status = 422, description = "Invalid import content for the selected format"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn import(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<ImportChapterTranslationInstr>,
) -> HttpResult<ImportChapterTranslationVal> {
    //
    usecase::chapter_port::import::import::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
        chapter_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/chapters/{chapter_id}/translations/export` — export response body.
///
/// `format` selects one or both generated documents as a comma-separated value.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/translations/export",
    tag = "chapter-port",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("format" = String, Query, description = "Comma-separated export formats: poprako,label_plus"),
    ),
    responses(
        (status = 200, description = "Selected translation exports", body = ExportChapterTranslationsVal, content_type = "application/json"),
        (status = 403, description = "No perm to export this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ExportChapterTranslationInstr>,
) -> Result<Response, HttpError> {
    //
    let payload =
        export_payload(&harn, user_token, chapter_id, instr.format).await?;

    body_response(payload)
}

/// `GET /api/v1/chapters/{chapter_id}/translations/export/download` — export as file download.
///
/// The downloaded JSON contains every document selected by `format`.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/translations/export/download",
    tag = "chapter-port",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("format" = String, Query, description = "Comma-separated export formats: poprako,label_plus"),
    ),
    responses(
        (status = 200, description = "Selected translation exports download", body = ExportChapterTranslationsVal, content_type = "application/json"),
        (status = 403, description = "No perm to export this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export_download(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ExportChapterTranslationInstr>,
) -> Result<Response, HttpError> {
    //
    let filename = format!("chapter_{}", chapter_id);

    let payload =
        export_payload(&harn, user_token, chapter_id, instr.format).await?;

    download_response(&filename, payload)
}

// Internal payload carrying the serialised export content and response metadata.
struct TranslationExportPayload {
    //
    // MIME type of the HTTP response body.
    content_type: &'static str,
    // File extension for the downloaded filename suffix.
    ext: &'static str,
    // Raw bytes of the serialised export payload.
    body: Bytes,
}

// Loads exported chapter data from the selected usecase path and builds the
// response payload that is later written into the HTTP body.
#[instrument(level = "info", skip_all)]
async fn export_payload(
    harn: &AppHarn,
    user_token: UserToken,
    chapter_id: String,
    formats: ExportFormatSpec,
) -> Result<TranslationExportPayload, HttpError> {
    //
    let val = usecase::chapter_port::export::export::<
        _,
        RdbContext<ReptRead>,
        HybRepo,
    >(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        chapter_id,
        formats,
    )
    .await?;

    let body = serde_json::to_vec(&val).map_err(|err| {
        //
        tracing::error!(
            operation = "serialize_chapter_export",
            sdk_err = ?err,
            "JSON SDK serialization error",
        );

        HttpError::internal()
    })?;

    Ok(TranslationExportPayload {
        content_type: "application/json",
        ext: "json",
        body: Bytes::from(body),
    })
}

// Builds a `200 OK` inline export response with the payload's MIME type.
fn body_response(
    payload: TranslationExportPayload,
) -> Result<Response, HttpError> {
    //
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, payload.content_type)
        .body(Body::from(payload.body))
        .map_err(|err| {
            //
            tracing::error!(
                operation = "build_inline_export_response",
                sdk_err = ?err,
                "HTTP SDK response build error",
            );

            HttpError::internal()
        })
}

// Builds a `200 OK` attachment response with MIME type and filename header.
fn download_response(
    filename_base: &str,
    payload: TranslationExportPayload,
) -> Result<Response, HttpError> {
    //
    let filename = format!("{}.{}", filename_base, payload.ext);

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
            tracing::error!(
                operation = "build_download_export_response",
                sdk_err = ?err,
                "HTTP SDK response build error",
            );

            HttpError::internal()
        })
}
