//! Team handlers: CRUD, avatar upload flow, and deletion.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;

use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;
#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::team_data;
use crate::model::user_model;
use crate::usecase;

/// `POST /api/v1/teams` — create a new team.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/teams",
    tag = "teams",
    request_body = team_data::CreateData,
    responses(
        (status = 201, description = "Team created", body = HttpBody<team_data::InfoVal>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Only super-admins can create teams"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<team_data::CreateData>,
) -> HttpResult<team_data::InfoVal> {
    usecase::team::create(
        harn.drive(),
        harn.repo(),
        harn.image_pool(),
        user_token,
        data,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams` — list teams with pagination.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/teams",
    tag = "teams",
    description = "Lists teams. Omit `user_id` to list all teams (super-admin only, otherwise `403`); supply `user_id` to list teams that user has joined. Examples: `/api/v1/teams?user_id=u_1&offset=0&limit=20`, `/api/v1/teams?offset=0&limit=20` (super-admin).",
    params(team_data::ListInfosData),
    responses(
        (status = 200, description = "Teams listed", body = HttpBody<Vec<team_data::InfoVal>>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Listing all teams requires super-admin"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Query(data): Query<team_data::ListInfosData>,
) -> HttpResult<Vec<team_data::InfoVal>> {
    usecase::team::list_infos(harn.repo(), harn.image_pool(), user_token, data)
        .await?
        .accept(StatusCode::OK)
}

/// `GET /api/v1/teams/{team_id}` — fetch a team by id.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 200, description = "Team info retrieved", body = HttpBody<team_data::InfoVal>),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
) -> HttpResult<team_data::InfoVal> {
    usecase::team::get_info(harn.repo(), harn.image_pool(), team_id)
        .await?
        .accept(StatusCode::OK)
}

/// `PUT /api/v1/teams/{team_id}` — update a team's profile.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    put,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = team_data::UpdateInfoData,
    responses(
        (status = 204, description = "Team updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<team_data::UpdateInfoData>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&team_id, &data.id)?;

    usecase::team::update_info(harn.repo(), user_token, data).await?;

    no_content()
}

/// `POST /api/v1/teams/{team_id}/avatar/reserve` — reserve a team avatar upload slot.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/avatar/reserve",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = team_data::ReserveAvatarData,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = HttpBody<team_data::ReserveAvatarVal>),
        (status = 403, description = "No permission to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn reserve_avatar(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<team_data::ReserveAvatarData>,
) -> HttpResult<team_data::ReserveAvatarVal> {
    usecase::team::reserve_avatar(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        team_id,
        data,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/teams/{team_id}/avatar/mark-uploaded` — confirm a team avatar upload.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/avatar/mark-uploaded",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = team_data::MarkAvatarUploadedData,
    responses(
        (status = 204, description = "Avatar upload confirmed"),
        (status = 403, description = "No permission to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn mark_avatar_uploaded(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<team_data::MarkAvatarUploadedData>,
) -> HttpNoContent {
    //
    usecase::team::mark_avatar_uploaded(harn.repo(), user_token, team_id, data)
        .await?;

    no_content()
}

/// `DELETE /api/v1/teams/{team_id}` — delete a team and all descendants.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
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
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
) -> HttpNoContent {
    //
    usecase::team::delete(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        user_token,
        team_id,
    )
    .await?;

    no_content()
}
