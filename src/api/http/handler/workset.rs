use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use poprako_util::page::Page;

use crate::api::http::result::{Accept as _, HttpError, HttpResult};
use crate::harness::Harness;
use crate::usecase_legacy;
use crate::usecase_legacy::data_object::workset::{
    WorksetCreateParams, WorksetCreateReply, WorksetInfo, WorksetUpdateParams,
};

#[utoipa::path(
    post,
    path = "/worksets",
    tag = "worksets",
    request_body = WorksetCreateParams,
    responses(
        (status = 201, description = "Workset created", body = WorksetCreateReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn create(
    State(harn): State<Harness>,
    Json(params): Json<WorksetCreateParams>,
) -> HttpResult<WorksetCreateReply> {
    let reply = usecase_legacy::workset::create(&harn, params).await?;

    reply.accept(StatusCode::CREATED)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct WorksetListQuery {
    pub team_id: String,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/worksets",
    tag = "worksets",
    params(WorksetListQuery),
    responses(
        (status = 200, description = "Worksets listed", body = Vec<WorksetInfo>),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<Harness>,
    Query(params): Query<WorksetListQuery>,
) -> HttpResult<Vec<WorksetInfo>> {
    let page = Page {
        offset: params.offset.unwrap_or(0) as usize,
        limit: params.limit.unwrap_or(20) as usize,
    };

    let infos = usecase_legacy::workset::list_infos(&harn, &params.team_id, page).await?;

    infos.accept(StatusCode::OK)
}

#[utoipa::path(
    put,
    path = "/worksets/{workset_id}",
    tag = "worksets",
    params(
        ("workset_id" = String, Path, description = "Workset ID")
    ),
    request_body = WorksetUpdateParams,
    responses(
        (status = 200, description = "Workset updated"),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn update_infos(
    State(harn): State<Harness>,
    Path(workset_id): Path<String>,
    Json(params): Json<WorksetUpdateParams>,
) -> HttpResult<()> {
    usecase_legacy::workset::update_info(&harn, workset_id, params).await?;

    ().accept(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/worksets/{workset_id}",
    tag = "worksets",
    params(
        ("workset_id" = String, Path, description = "Workset ID")
    ),
    responses(
        (status = 200, description = "Workset deleted"),
        (status = 400, description = "Workset not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn delete(State(harn): State<Harness>, Path(workset_id): Path<String>) -> HttpResult<()> {
    usecase_legacy::workset::delete(&harn, workset_id).await?;

    ().accept(StatusCode::OK)
}
