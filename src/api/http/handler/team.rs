use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;

use poprako_util::page::Page;

use crate::api::http::result::{Accept as _, HttpError, HttpResult};
use crate::harness::Harness;
use crate::usecase_legacy;
use crate::usecase_legacy::data_object::team::{
    AvatarMarkUploadedParams, AvatarReserveParams, AvatarReserveReply, CreateParams,
    InfoUpdateParams, TeamInfo,
};

#[utoipa::path(
    post,
    path = "/teams",
    tag = "teams",
    request_body = CreateParams,
    responses(
        (status = 201, description = "Team created", body = TeamInfo),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn create(
    State(harn): State<Harness>,
    Json(params): Json<CreateParams>,
) -> HttpResult<TeamInfo> {
    let info = usecase_legacy::team::create(&harn, params).await?;

    info.accept(StatusCode::CREATED)
}

#[utoipa::path(
    get,
    path = "/teams/{team_id}",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team info retrieved", body = TeamInfo),
        (status = 400, description = "Team not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn get_info(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
) -> HttpResult<TeamInfo> {
    let info = usecase_legacy::team::get_info(&harn, &team_id).await?;

    info.accept(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/teams",
    tag = "teams",
    params(
        ("offset" = Option<i64>, Query, description = "Pagination offset"),
        ("limit" = Option<i64>, Query, description = "Pagination limit")
    ),
    responses(
        (status = 200, description = "Teams listed", body = Vec<TeamInfo>),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<Harness>,
    axum::extract::Query(params): axum::extract::Query<TeamListQuery>,
) -> HttpResult<Vec<TeamInfo>> {
    let page = Page {
        offset: params.offset.unwrap_or(0) as usize,
        limit: params.limit.unwrap_or(20) as usize,
    };

    let infos = usecase_legacy::team::list_infos(&harn, page).await?;

    infos.accept(StatusCode::OK)
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TeamListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[utoipa::path(
    put,
    path = "/teams/{team_id}",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    request_body = InfoUpdateParams,
    responses(
        (status = 200, description = "Team updated"),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn update_info(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
    Json(params): Json<InfoUpdateParams>,
) -> HttpResult<()> {
    usecase_legacy::team::update_info(&harn, team_id, params).await?;

    ().accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/teams/{team_id}/avatar/reserve",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    request_body = AvatarReserveParams,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = AvatarReserveReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn reserve_avatar(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
    Json(params): Json<AvatarReserveParams>,
) -> HttpResult<AvatarReserveReply> {
    let reply = usecase_legacy::team::reserve_avatar(&harn, team_id, params).await?;

    reply.accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/teams/{team_id}/avatar/mark-uploaded",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    request_body = AvatarMarkUploadedParams,
    responses(
        (status = 200, description = "Avatar upload confirmed"),
        (status = 400, description = "Team not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn mark_avatar_uploaded(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
    Json(params): Json<AvatarMarkUploadedParams>,
) -> HttpResult<()> {
    usecase_legacy::team::mark_avatar_uploaded(&harn, team_id, params).await?;

    ().accept(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/teams/{team_id}",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team deleted"),
        (status = 400, description = "Team not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn delete(State(harn): State<Harness>, Path(team_id): Path<String>) -> HttpResult<()> {
    usecase_legacy::team::delete(&harn, team_id).await?;

    ().accept(StatusCode::OK)
}
