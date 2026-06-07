use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use poprako_util::page::Page;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpError;
use crate::api::http::result::HttpResult;
use crate::harness::Harness;
use crate::usecase;
use crate::usecase::data_object::team::{
    ReserveTeamAvatarParams, ReserveTeamAvatarReply, TeamBase, TeamCreateParams,
    TeamInfoUpdateParams,
};

#[utoipa::path(
    post,
    path = "/teams",
    tag = "teams",
    request_body = TeamCreateParams,
    responses(
        (status = 201, description = "Team created", body = TeamBase),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
pub async fn create(
    State(harn): State<Harness>,
    Json(params): Json<TeamCreateParams>,
) -> HttpResult<TeamBase> {
    let base = usecase::team::create(&harn, params).await?;

    base.accept(StatusCode::CREATED)
}

#[utoipa::path(
    get,
    path = "/teams/{team_id}",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Team info retrieved", body = TeamBase),
        (status = 400, description = "Team not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
pub async fn get_info(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
) -> HttpResult<TeamBase> {
    let base = usecase::team::get_info(&harn, &team_id).await?;

    base.accept(StatusCode::OK)
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
        (status = 200, description = "Teams listed", body = Vec<TeamBase>),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
pub async fn list(
    State(harn): State<Harness>,
    axum::extract::Query(params): axum::extract::Query<TeamListQuery>,
) -> HttpResult<Vec<TeamBase>> {
    let page = Page {
        offset: params.offset.unwrap_or(0) as usize,
        limit: params.limit.unwrap_or(20) as usize,
    };

    let bases = usecase::team::list(&harn, page).await?;

    bases.accept(StatusCode::OK)
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
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
    request_body = TeamInfoUpdateParams,
    responses(
        (status = 200, description = "Team updated"),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
pub async fn update(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
    Json(params): Json<TeamInfoUpdateParams>,
) -> HttpResult<()> {
    usecase::team::update_info(&harn, team_id, params).await?;

    ().accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/teams/{team_id}/avatar/reserve",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    request_body = ReserveTeamAvatarParams,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = ReserveTeamAvatarReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
pub async fn reserve_avatar(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
    Json(params): Json<ReserveTeamAvatarParams>,
) -> HttpResult<ReserveTeamAvatarReply> {
    let reply = usecase::team::reserve_avatar(&harn, team_id, params).await?;

    reply.accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/teams/{team_id}/avatar/mark-uploaded",
    tag = "teams",
    params(
        ("team_id" = String, Path, description = "Team ID")
    ),
    responses(
        (status = 200, description = "Avatar upload confirmed"),
        (status = 400, description = "Team not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
pub async fn mark_avatar_uploaded(
    State(harn): State<Harness>,
    Path(team_id): Path<String>,
) -> HttpResult<()> {
    usecase::team::mark_avatar_uploaded(&harn, team_id).await?;

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
pub async fn delete(State(harn): State<Harness>, Path(team_id): Path<String>) -> HttpResult<()> {
    usecase::team::delete(&harn, team_id).await?;

    ().accept(StatusCode::OK)
}
