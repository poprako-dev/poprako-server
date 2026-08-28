//! Native terminology-base import and export handlers.

#[cfg(test)]
// Tests for native terminology-base query and wire contracts.
mod tests;

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;
use serde::Deserialize;
use tracing::instrument;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{Accept as _, HttpError, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::instr::termbase_port::ImportTermbaseInstr;
use crate::data::val::termbase_port::{ExportTermbaseVal, ImportTermbaseVal};
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::termbase::TermbaseScope;

/// Query controls for terminology-base import.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ImportTermbaseQuery {
    /// Merge into a same-name terminology base instead of rejecting it.
    #[serde(default)]
    pub force_merge: bool,
}

/// `GET /api/v1/termbases/{termbase_id}/export` — export native JSON.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}/export",
    tag = "termbase-port",
    params(("termbase_id" = String, Path, description = "Termbase ID")),
    responses(
        (status = 200, description = "Native terminology-base export", body = ExportTermbaseVal, content_type = "application/json"),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> Result<Response, HttpError> {
    //
    let body = export_payload(&harn, user_token, termbase_id.clone()).await?;

    export_response(&termbase_id, body, false)
}

/// `GET /api/v1/termbases/{termbase_id}/export/download` — download native JSON.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}/export/download",
    tag = "termbase-port",
    params(("termbase_id" = String, Path, description = "Termbase ID")),
    responses(
        (status = 200, description = "Native terminology-base download", body = ExportTermbaseVal, content_type = "application/json"),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export_download(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> Result<Response, HttpError> {
    //
    let body = export_payload(&harn, user_token, termbase_id.clone()).await?;

    export_response(&termbase_id, body, true)
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
    request_body = ImportTermbaseInstr,
    responses(
        (status = 200, description = "Termbase merged", body = HttpBody<ImportTermbaseVal>),
        (status = 201, description = "Termbase imported", body = HttpBody<ImportTermbaseVal>),
        (status = 403, description = "Team translator or proofreader required"),
        (status = 422, description = "Invalid document, capacity, or duplicate name"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn import_team(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ImportTermbaseQuery>,
    Json(instr): Json<ImportTermbaseInstr>,
) -> HttpResult<ImportTermbaseVal> {
    //
    let import_termbase_val =
        usecase::termbase_port::import::<_, RdbContext<ReptRead>, HybRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            TermbaseScope::Team { team_id },
            query.force_merge,
            instr,
        )
        .await?;

    let status = if import_termbase_val.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    import_termbase_val.accept(status)
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
    request_body = ImportTermbaseInstr,
    responses(
        (status = 200, description = "Termbase merged", body = HttpBody<ImportTermbaseVal>),
        (status = 201, description = "Termbase imported", body = HttpBody<ImportTermbaseVal>),
        (status = 403, description = "Team translator or proofreader required"),
        (status = 422, description = "Invalid document, capacity, or duplicate name"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn import_comic(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ImportTermbaseQuery>,
    Json(instr): Json<ImportTermbaseInstr>,
) -> HttpResult<ImportTermbaseVal> {
    //
    let import_termbase_val =
        usecase::termbase_port::import::<_, RdbContext<ReptRead>, HybRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            TermbaseScope::Comic { comic_id },
            query.force_merge,
            instr,
        )
        .await?;

    let status = if import_termbase_val.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };

    import_termbase_val.accept(status)
}

// Load and serialize one export document for either response mode.
async fn export_payload(
    harn: &AppHarn,
    user_token: UserToken,
    termbase_id: String,
) -> Result<Bytes, HttpError> {
    //
    let export_termbase_val =
        usecase::termbase_port::export::<_, RdbContext<ReptRead>, HybRepo>(
            (harn.nucl().rept_read(), harn.repo()),
            user_token,
            termbase_id,
        )
        .await?;

    serialize_export(&export_termbase_val)
}

// Build either an inline or attachment export response.
fn export_response(
    termbase_id: &str,
    body: Bytes,
    download: bool,
) -> Result<Response, HttpError> {
    //
    let builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json");

    let builder = if download {
        //
        builder.header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"termbase_{termbase_id}.json\""),
        )
    } else {
        builder
    };

    builder.body(Body::from(body)).map_err(|err| {
        //
        tracing::error!(
            operation = "build_termbase_export_response",
            sdk_err = ?err,
            "HTTP SDK response build error",
        );

        HttpError::internal()
    })
}

// Serialize one native export document at the HTTP SDK boundary.
fn serialize_export(
    export_termbase_val: &ExportTermbaseVal,
) -> Result<Bytes, HttpError> {
    //
    let body = serde_json::to_vec(export_termbase_val).map_err(|err| {
        //
        tracing::error!(
            operation = "serialize_termbase_export",
            sdk_err = ?err,
            "JSON SDK serialization error",
        );

        HttpError::internal()
    })?;

    Ok(Bytes::from(body))
}
