//! Native terminology-base import and export handlers.
#[cfg(test)]
// Tests for native terminology-base query and wire contracts.
mod tests;

use axum::body::Bytes;
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use tracing::instrument;

use crate::api::http::object::termbase_port::{
    AppendTermbaseBody, AppendTermbasePayload, ExportTermbasePayload,
    ImportTermbaseBody, ImportTermbasePayload, ImportTermbaseQuery,
};
use crate::api::http::result::{
    AcceptInto as _, HttpBody, HttpError, HttpRawBody, HttpResult,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::termbase_port as termbase_port_usecase;
use crate::value::termbase::TermbaseScope;

/// `GET /api/v1/termbases/{termbase_id}/export` — export native JSON.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}/export",
    tag = "termbase-port",
    params(("termbase_id" = String, Path, description = "Termbase ID")),
    responses(
        (status = 200, description = "Native terminology-base export",
            body = ExportTermbasePayload, content_type = "application/json"),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<HttpRawBody> {
    //
    let body = export_payload(&harn, user_token, termbase_id).await?;

    Ok(export_body(body))
}

/// `GET /api/v1/termbases/{termbase_id}/export/download` — download native JSON.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}/export/download",
    tag = "termbase-port",
    params(("termbase_id" = String, Path, description = "Termbase ID")),
    responses(
        (status = 200, description = "Native terminology-base download",
            body = ExportTermbasePayload, content_type = "application/json"),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export_download(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<HttpRawBody> {
    //
    let body = export_payload(&harn, user_token, termbase_id.clone()).await?;

    let disposition = HeaderValue::from_str(
        &["attachment; filename=\"termbase_", &termbase_id, ".json\""].concat(),
    )
    .map_err(|error| {
        //
        tracing::error!(
            operation = "build_termbase_export_disposition",
            sdk_error = ?error,
            "HTTP SDK export filename error",
        );

        HttpError::internal()
    })?;

    Ok(export_body(body).with_header(header::CONTENT_DISPOSITION, disposition))
}

/// `POST /api/v1/teams/{team_id}/termbases/import` — import into a team scope.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/termbases/import",
    tag = "termbase-port",
    params(
        ("team_id" = String, Path, description = "Team ID"),
        ImportTermbaseQuery,
    ),
    request_body = ImportTermbaseBody,
    responses(
        (status = 200, description = "Termbase merged",
            body = HttpBody<ImportTermbasePayload>),
        (status = 201, description = "Termbase imported",
            body = HttpBody<ImportTermbasePayload>),
        (status = 403, description = "Team translator or proofreader required"),
        (status = 422, description = "Invalid document, capacity, or \
        duplicate name"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn import_team(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ImportTermbaseQuery>,
    Json(instr): Json<ImportTermbaseBody>,
) -> HttpResult<ImportTermbasePayload> {
    //
    let import_termbase_val =
        termbase_port_usecase::import::<_, RdbContext<ReptRead>, RdbRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            TermbaseScope::Team { team_id },
            query.force_merge,
            instr.into(),
        )
        .await?;

    if import_termbase_val.created {
        return Ok(import_termbase_val.accept_into(StatusCode::CREATED));
    }

    Ok(import_termbase_val.accept_into(StatusCode::OK))
}

/// `POST /api/v1/comics/{comic_id}/termbases/import` — import into a comic scope.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/termbases/import",
    tag = "termbase-port",
    params(
        ("comic_id" = String, Path, description = "Comic ID"),
        ImportTermbaseQuery,
    ),
    request_body = ImportTermbaseBody,
    responses(
        (status = 200, description = "Termbase merged",
            body = HttpBody<ImportTermbasePayload>),
        (status = 201, description = "Termbase imported",
            body = HttpBody<ImportTermbasePayload>),
        (status = 403, description = "Team translator or proofreader required"),
        (status = 422, description = "Invalid document, capacity, or \
        duplicate name"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn import_comic(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ImportTermbaseQuery>,
    Json(instr): Json<ImportTermbaseBody>,
) -> HttpResult<ImportTermbasePayload> {
    //
    let import_termbase_val =
        termbase_port_usecase::import::<_, RdbContext<ReptRead>, RdbRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            TermbaseScope::Comic { comic_id },
            query.force_merge,
            instr.into(),
        )
        .await?;

    if import_termbase_val.created {
        return Ok(import_termbase_val.accept_into(StatusCode::CREATED));
    }

    Ok(import_termbase_val.accept_into(StatusCode::OK))
}

/// `POST /api/v1/termbases/{termbase_id}/append` — atomically append portable terms.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/termbases/{termbase_id}/append",
    tag = "termbase-port",
    description = "Append a strict terms-only document. Match trimmed, \
    case-insensitive sources, preserve existing source spelling and \
    translation order, and append distinct translations. Comments are compared \
    as whole text blocks and joined with a newline when different. Empty or \
    unchanged batches preserve timestamps. This operation is not idempotent \
    when different comment blocks are combined; do not blindly replay after an \
    uncertain response.",
    params(("termbase_id" = String, Path,
        description = "Existing termbase ID")),
    request_body = AppendTermbaseBody,
    responses(
        (status = 200, description = "Atomic \
        append counts", body = HttpBody<AppendTermbasePayload>),
        (status = 403, description = "Team translator or proofreader required"),
        (status = 422, description = "Invalid document, missing termbase, or \
        capacity exceeded"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn append(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<AppendTermbaseBody>,
) -> HttpResult<AppendTermbasePayload> {
    //
    let append_termbase_val =
        termbase_port_usecase::append::<_, RdbContext<ReptRead>, RdbRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            termbase_id,
            instr.into(),
        )
        .await?;

    Ok(append_termbase_val.accept_into(StatusCode::OK))
}

// Load and serialize one export document for either response mode.
async fn export_payload(
    harn: &AppHarn,
    user_token: UserToken,
    termbase_id: String,
) -> Result<Bytes, HttpError> {
    //
    let export_termbase_val =
        termbase_port_usecase::export::<_, RdbContext<ReptRead>, RdbRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            termbase_id,
        )
        .await?;

    let export_payload = ExportTermbasePayload::from(export_termbase_val);

    serialize_export(&export_payload)
}

// Build the shared inline export body.
fn export_body(body: Bytes) -> HttpBody<HttpRawBody> {
    //
    HttpBody::new(StatusCode::OK, HttpRawBody::new(body)).with_header(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    )
}

// Serialize one native export document at the HTTP SDK boundary.
fn serialize_export(
    export_termbase_val: &ExportTermbasePayload,
) -> Result<Bytes, HttpError> {
    //
    let body = serde_json::to_vec(export_termbase_val).map_err(|error| {
        //
        tracing::error!(
            operation = "serialize_termbase_export",
            sdk_error = ?error,
            "JSON SDK serialization error",
        );

        HttpError::internal()
    })?;

    Ok(Bytes::from(body))
}
