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
use crate::data::instr::team::{
    CreateTeamInstr, ListTeamInfosInstr, MarkTeamAvatarUploadedInstr,
    ReserveTeamAvatarInstr, UpdateTeamInfoInstr,
};
use crate::data::val::team::ReserveTeamAvatarVal;
use crate::data::view::team::TeamInfoView;
use crate::model::shared::user::UserToken;
use crate::usecase;

/// `POST /api/v1/teams` — create a new team.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/teams",
    tag = "teams",
    request_body = CreateTeamInstr,
    responses(
        (status = 201, description = "Team created", body = HttpBody<TeamInfoView>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Only super-admins can create teams"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateTeamInstr>,
) -> HttpResult<TeamInfoView> {
    usecase::team::create(
        (harn.nucl(), harn.repo(), harn.image_pool()),
        user_token,
        instr,
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
    params(ListTeamInfosInstr),
    responses(
        (status = 200, description = "Teams listed", body = HttpBody<Vec<TeamInfoView>>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Listing all teams requires super-admin"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ListTeamInfosInstr>,
) -> HttpResult<Vec<TeamInfoView>> {
    usecase::team::list_infos(
        (harn.repo(), harn.image_pool()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/teams/{team_id}/online-users` — list active team users.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/online-users",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 200, description = "Online user IDs listed", body = HttpBody<Vec<String>>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Team membership required"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_online_user_ids(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<Vec<String>> {
    usecase::team::list_online_user_ids((harn.repo(),), user_token, team_id)
        .await?
        .accept(StatusCode::OK)
}

/// `PUT /api/v1/teams/{team_id}/mark-self-online` — refresh own lease.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/teams/{team_id}/mark-self-online",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 204, description = "Online lease refreshed"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Team membership required"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_self_online(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::team::mark_self_online((harn.repo(),), user_token, team_id)
        .await?;

    no_content()
}

/// `GET /api/v1/teams/{team_id}` — fetch a team by id.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    responses(
        (status = 200, description = "Team info retrieved", body = HttpBody<TeamInfoView>),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
) -> HttpResult<TeamInfoView> {
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
    request_body = UpdateTeamInfoInstr,
    responses(
        (status = 204, description = "Team updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No perm to update this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateTeamInfoInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&team_id, &instr.id)?;

    usecase::team::update_info((harn.repo(),), user_token, instr).await?;

    no_content()
}

/// `POST /api/v1/teams/{team_id}/avatar/reserve` — reserve a team avatar upload slot.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/teams/{team_id}/avatar/reserve",
    tag = "teams",
    params(("team_id" = String, Path, description = "Team ID")),
    request_body = ReserveTeamAvatarInstr,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = HttpBody<ReserveTeamAvatarVal>),
        (status = 403, description = "No perm to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn reserve_avatar(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<ReserveTeamAvatarInstr>,
) -> HttpResult<ReserveTeamAvatarVal> {
    usecase::team::reserve_avatar(
        (harn.nucl(), harn.repo(), harn.prom(), harn.image_pool()),
        user_token,
        team_id,
        instr,
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
    request_body = MarkTeamAvatarUploadedInstr,
    responses(
        (status = 204, description = "Avatar upload confirmed"),
        (status = 403, description = "No perm to modify this team's avatar"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_avatar_uploaded(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkTeamAvatarUploadedInstr>,
) -> HttpNoContent {
    //
    usecase::team::mark_avatar_uploaded(
        (harn.nucl(), harn.repo(), harn.image_pool()),
        user_token,
        team_id,
        instr,
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
        (status = 403, description = "No perm to delete this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::team::delete(
        (harn.nucl(), harn.repo(), harn.prom()),
        user_token,
        team_id,
    )
    .await?;

    no_content()
}
