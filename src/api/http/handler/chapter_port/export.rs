//! Shared chapter translation export response helpers.

use axum::body::Bytes;
use axum::http::{HeaderValue, StatusCode, header};
use tracing::instrument;

use crate::api::http::result::{HttpBody, HttpError, HttpRawBody, HttpResult};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::chapter_port::export_translation as chapter_port_export_translation_usecase;
use crate::value::chapter_port::ExportFormatSpec;

/// Loads and serializes one export document.
#[instrument(level = "info", skip_all)]
pub async fn export_payload(
    harn: &AppHarn,
    user_token: UserToken,
    chapter_id: String,
    formats: ExportFormatSpec,
    with_raw_ident: bool,
) -> Result<Bytes, HttpError> {
    //
    let val = chapter_port_export_translation_usecase::export_translation::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
    >(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        user_token,
        chapter_id,
        formats,
        with_raw_ident,
    )
    .await?;

    let body = serde_json::to_vec(&val).map_err(|error| {
        //
        tracing::error!(
            operation = "serialize_chapter_export",
            sdk_error = ?error,
            "JSON SDK serialization error",
        );

        HttpError::internal()
    })?;

    Ok(Bytes::from(body))
}

/// Builds a `200 OK` inline JSON export response.
pub fn body_response(body: Bytes) -> HttpBody<HttpRawBody> {
    //
    HttpBody::new(StatusCode::OK, HttpRawBody::new(body)).with_header(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    )
}

/// Builds a `200 OK` JSON attachment response with a filename header.
pub fn download_response(
    filename_base: &str,
    body: Bytes,
) -> HttpResult<HttpRawBody> {
    //
    let filename = [filename_base, ".json"].concat();

    let disposition = HeaderValue::from_str(
        &["attachment; filename=\"", &filename, "\""].concat(),
    )
    .map_err(|error| {
        //
        tracing::error!(
            operation = "build_chapter_export_disposition",
            sdk_error = ?error,
            "HTTP SDK export filename error",
        );

        HttpError::internal()
    })?;

    Ok(HttpBody::new(StatusCode::OK, HttpRawBody::new(body))
        .with_header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )
        .with_header(header::CONTENT_DISPOSITION, disposition))
}
