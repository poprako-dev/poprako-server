//! Team handlers: CRUD, avatar upload flow, and deletion.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;
#[allow(unused_imports)]
use crate::api::http::result::{Accept as _, HttpBody, HttpNoContent, HttpResult, no_content};
use crate::api::http::state::AppHarn;
use crate::data::team::{CreateTeamParams, ListTeamInfosParams, MarkTeamAvatarUploadedParams, ReserveTeamAvatarParams, ReserveTeamAvatarPayload, TeamInfoVal, UpdateTeamInfoParams};
use crate::model::user::UserToken;
use crate::usecase;

/// `POST /api/v1/teams` — create a new team.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/teams",
    tag = "teams",
    request_body = CreateTeamParams,
    responses(
        (status = 201, description = "Team created", body = HttpBody<TeamInfoVal>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Only super-admins can create teams"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<CreateTeamParams>,
) -> HttpResult<TeamInfoVal> {
    usecase::team::create(
        (harn.drive(), harn.repo(), harn.image_pool()),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams` — list teams with pagination.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams",
    tag = "teams",
    description = "Lists teams. Omit `user_id` to list all teams (super-admin only, otherwise `403`); supply `user_id` to list teams that user has joined. Examples: `/api/v1/teams?user_id=u_1&offset=0&limit=20`, `/api/v1/teams?offset=0&limit=20` (super-admin).",
    params(ListTeamInfosParams),
    responses(
        (status = 200, description = "Teams listed", body = HttpBody<Vec<TeamInfoVal>>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Listing all teams requires super-admin"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<ListTeamInfosParams>,
) -> HttpResult<Vec<TeamInfoVal>> {
    usecase::team::list_infos(
        (harn.repo(), harn.image_pool()),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/teams/{team_id}` — fetch a team by id.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 200, description = "Team info retrieved", body = HttpBody<TeamInfoVal>),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
) -> HttpResult<TeamInfoVal> {
    usecase::team::get_info((harn.repo(), harn.image_pool()), team_id)
        .await?
        .accept(StatusCode::OK)
}

/// `PUT /api/v1/teams/{team_id}` — update a team's profile.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = UpdateTeamInfoParams,
    responses(
        (status = 204, description = "Team updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<UpdateTeamInfoParams>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&team_id, &params.id)?;

    usecase::team::update_info((harn.repo(),), user_token, params).await?;

    no_content()
}

/// `POST /api/v1/teams/{team_id}/avatar/reserve` — reserve a team avatar upload slot.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/avatar/reserve",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = ReserveTeamAvatarParams,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = HttpBody<ReserveTeamAvatarPayload>),
        (status = 403, description = "No permission to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_avatar(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<ReserveTeamAvatarParams>,
) -> HttpResult<ReserveTeamAvatarPayload> {
    usecase::team::reserve_avatar(
        (harn.drive(), harn.repo(), harn.prom(), harn.image_pool()),
        user_token,
        team_id,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/teams/{team_id}/avatar/mark-uploaded` — confirm a team avatar upload.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/avatar/mark-uploaded",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = MarkTeamAvatarUploadedParams,
    responses(
        (status = 204, description = "Avatar upload confirmed"),
        (status = 403, description = "No permission to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn mark_avatar_uploaded(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<MarkTeamAvatarUploadedParams>,
) -> HttpNoContent {
    //
    usecase::team::mark_avatar_uploaded(
        (harn.drive(), harn.repo(), harn.image_pool()),
        user_token,
        team_id,
        params,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/teams/{team_id}` — delete a team and all descendants.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 204, description = "Team deleted"),
        (status = 403, description = "No permission to delete this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::team::delete(
        (harn.drive(), harn.repo(), harn.prom()),
        user_token,
        team_id,
    )
    .await?;

    no_content()
}
