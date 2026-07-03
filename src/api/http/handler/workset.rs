//! Workset handlers: create, list, read, update, and delete.

use axum::Json;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;

use tracing::instrument;

use crate::api::http::handler::util::Pagination;
use crate::api::http::handler::util::ensure_path_matches_body_id;
use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpNoContent;
use crate::api::http::result::HttpResult;
use crate::api::http::result::no_content;
use crate::api::http::state::AppHarn;
use crate::data::workset::{
    CreateWorksetData, CreateWorksetVal, ListWorksetInfosData, UpdateWorksetInfoData,
    WorksetInfoVal,
};
use crate::model::user::UserToken;
use crate::usecase;

/// `POST /api/v1/worksets` — create a workset inside a team.
#[utoipa::path(
    post,
    path = "/api/v1/worksets",
    tag = "worksets",
    request_body = CreateWorksetData,
    responses(
        (status = 201, description = "Workset created", body = CreateWorksetVal),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "No permission to create worksets in this team"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<CreateWorksetData>,
) -> HttpResult<CreateWorksetVal> {
    let reply = usecase::workset::create(harn.drive(), harn.repo(), user_token, data).await?;
    reply.accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/worksets` — list worksets in a team.
#[utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/worksets",
    tag = "worksets",
    params(("team_id" = String, Path, description = "Team ID"), Pagination),
    responses(
        (status = 200, description = "Worksets listed", body = Vec<WorksetInfoVal>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "No permission to list worksets in this team"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(pagination): Query<Pagination>,
) -> HttpResult<Vec<WorksetInfoVal>> {
    let data = ListWorksetInfosData {
        team_id,
        offset: pagination.offset,
        limit: pagination.limit,
    };

    let infos = usecase::workset::list_infos(harn.repo(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}

/// `GET /api/v1/worksets/{workset_id}` — fetch a workset by id.
#[utoipa::path(
    get,
    path = "/api/v1/worksets/{workset_id}",
    tag = "worksets",
    params(("workset_id" = String, Path, description = "Workset ID")),
    responses(
        (status = 200, description = "Workset info retrieved", body = WorksetInfoVal),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "No permission to view this workset"),
        (status = 404, description = "Workset not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<WorksetInfoVal> {
    let info = usecase::workset::get_info(harn.repo(), user_token, workset_id).await?;
    info.accept(StatusCode::OK)
}

/// `PUT /api/v1/worksets/{workset_id}` — update a workset's profile.
#[utoipa::path(
    put,
    path = "/api/v1/worksets/{workset_id}",
    tag = "worksets",
    params(("workset_id" = String, Path, description = "Workset ID")),
    request_body = UpdateWorksetInfoData,
    responses(
        (status = 204, description = "Workset updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this workset"),
        (status = 404, description = "Workset not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<UpdateWorksetInfoData>,
) -> HttpNoContent {
    ensure_path_matches_body_id(&workset_id, &data.id)?;

    usecase::workset::update_info(harn.repo(), user_token, data).await?;

    no_content()
}

/// `DELETE /api/v1/worksets/{workset_id}` — delete a workset and descendants.
#[utoipa::path(
    delete,
    path = "/api/v1/worksets/{workset_id}",
    tag = "worksets",
    params(("workset_id" = String, Path, description = "Workset ID")),
    responses(
        (status = 204, description = "Workset deleted"),
        (status = 403, description = "No permission to delete this workset"),
        (status = 404, description = "Workset not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    usecase::workset::delete(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        user_token,
        workset_id,
    )
    .await?;
    no_content()
}
