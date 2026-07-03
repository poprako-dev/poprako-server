//! Team handlers: CRUD, avatar upload flow, and deletion.

use axum::Json;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;

use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;
use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpNoContent;
use crate::api::http::result::HttpResult;
use crate::api::http::result::NoContent;
use crate::api::http::state::AppHarn;
use crate::data::team::{
    CreateTeamData, ListTeamInfosData, MarkTeamAvatarUploadedData, ReserveTeamAvatarData,
    ReserveTeamAvatarVal, TeamInfoVal, UpdateTeamInfoData,
};
use crate::model::user::UserToken;
use crate::usecase;

/// `POST /api/v1/teams` — create a new team.
#[utoipa::path(
    post,
    path = "/api/v1/teams",
    tag = "teams",
    request_body = CreateTeamData,
    responses(
        (status = 201, description = "Team created", body = TeamInfoVal),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Only super-admins can create teams"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<CreateTeamData>,
) -> HttpResult<TeamInfoVal> {
    let info = usecase::team::create(harn.repo(), harn.image_pool(), user_token, data).await?;

    info.accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams` — list teams with pagination.
#[utoipa::path(
    get,
    path = "/api/v1/teams",
    tag = "teams",
    params(ListTeamInfosData),
    responses(
        (status = 200, description = "Teams listed", body = Vec<TeamInfoVal>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Listing all teams requires super-admin"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(data): Query<ListTeamInfosData>,
) -> HttpResult<Vec<TeamInfoVal>> {
    let infos = usecase::team::list_infos(harn.repo(), harn.image_pool(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}

/// `GET /api/v1/teams/{team_id}` — fetch a team by id.
#[utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 200, description = "Team info retrieved", body = TeamInfoVal),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
) -> HttpResult<TeamInfoVal> {
    let info = usecase::team::get_info(harn.repo(), harn.image_pool(), team_id).await?;

    info.accept(StatusCode::OK)
}

/// `PUT /api/v1/teams/{team_id}` — update a team's profile.
#[utoipa::path(
    put,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = UpdateTeamInfoData,
    responses(
        (status = 204, description = "Team updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this team"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<UpdateTeamInfoData>,
) -> HttpNoContent {
    ensure_path_matches_body_id(&team_id, &data.id)?;

    usecase::team::update_info(harn.repo(), user_token, data).await?;

    Ok(NoContent)
}

/// `POST /api/v1/teams/{team_id}/avatar/reserve` — reserve a team avatar upload slot.
#[utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/avatar/reserve",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = ReserveTeamAvatarData,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = ReserveTeamAvatarVal),
        (status = 403, description = "No permission to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn reserve_avatar(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<ReserveTeamAvatarData>,
) -> HttpResult<ReserveTeamAvatarVal> {
    let reply = usecase::team::reserve_avatar(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        team_id,
        data,
    )
    .await?;

    reply.accept(StatusCode::OK)
}

/// `POST /api/v1/teams/{team_id}/avatar/mark-uploaded` — confirm a team avatar upload.
#[utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/avatar/mark-uploaded",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = MarkTeamAvatarUploadedData,
    responses(
        (status = 204, description = "Avatar upload confirmed"),
        (status = 403, description = "No permission to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn mark_avatar_uploaded(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<MarkTeamAvatarUploadedData>,
) -> HttpNoContent {
    usecase::team::mark_avatar_uploaded(harn.repo(), user_token, team_id, data).await?;

    Ok(NoContent)
}

/// `DELETE /api/v1/teams/{team_id}` — delete a team and all descendants.
#[utoipa::path(
    delete,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 204, description = "Team deleted"),
        (status = 403, description = "No permission to delete this team"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    usecase::team::delete(harn.drive(), harn.repo(), harn.prom(), user_token, team_id).await?;

    Ok(NoContent)
}
