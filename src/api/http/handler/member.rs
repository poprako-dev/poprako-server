//! Member handlers: create, join, list, role update, and deletion.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;

use serde::Deserialize;

use tracing::instrument;

#[cfg(feature = "swagger-ui")]
use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;
#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::member_data;
use crate::model::user_model;
use crate::usecase;
use crate::value::member::MemberInclOpt;

/// Query for the current-user memberships list endpoint (`/members/me`).
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&incl=team&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct MemberMeListQuery {
    /// Related rows to embed. Repeatable. Values: `user`, `team`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<MemberInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/members` — create a member under a team.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/members",
    tag = "members",
    request_body = member_data::CreateData,
    responses(
        (status = 201, description = "Member created", body = HttpBody<member_data::CreateVal>),
        (status = 403, description = "No permission to create members in this team"),
        (status = 404, description = "User or team not found"),
        (status = 409, description = "User is already a member"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<member_data::CreateData>,
) -> HttpResult<member_data::CreateVal> {
    usecase::member::create(harn.drive(), harn.repo(), user_token, data)
        .await?
        .accept(StatusCode::CREATED)
}

/// `GET /api/v1/members` — list members by team or owner.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/members",
    tag = "members",
    description = "Lists members. Exactly one of `owner_id` or `team_id` is required. In `owner_id` mode, `role` and `fuzzy_nickname` must be omitted. In `team_id` mode, `fuzzy_nickname` and `role` are optional. `incl` embeds related rows. Examples: `/api/v1/members?team_id=t_1&fuzzy_nickname=al&role=1&incl=user`, `/api/v1/members?owner_id=u_1&incl=team`.",
    params(member_data::ListInfosData),
    responses(
        (status = 200, description = "Members listed", body = HttpBody<Vec<member_data::InfoVal>>),
        (status = 422, description = "Exactly one of owner_id or team_id is required, or owner_id combined with role/fuzzy_nickname"),
        (status = 403, description = "No permission to list members in this team"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Query(data): Query<member_data::ListInfosData>,
) -> HttpResult<Vec<member_data::InfoVal>> {
    usecase::member::list_infos(
        harn.repo(),
        harn.image_pool(),
        user_token,
        data,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/members/me` — list the current user's memberships.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/members/me",
    tag = "members",
    params(MemberMeListQuery),
    responses(
        (status = 200, description = "Current user memberships", body = HttpBody<Vec<member_data::InfoVal>>),
        (status = 401, description = "Authentication required"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_my_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Query(query): Query<MemberMeListQuery>,
) -> HttpResult<Vec<member_data::InfoVal>> {
    //
    let data = member_data::ListInfosData {
        owner_id: Some(user_token.user_id.clone()),
        team_id: None,
        fuzzy_nickname: None,
        role: None,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::member::list_infos(
        harn.repo(),
        harn.image_pool(),
        user_token,
        data,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/members/{member_id}/roles` — update a member's roles.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    put,
    path = "/api/v1/members/{member_id}/roles",
    tag = "members",
    params(("member_id" = String, Path, description = "Member ID")),
    request_body = member_data::UpdateRolesData,
    responses(
        (status = 204, description = "Member roles updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this member"),
        (status = 404, description = "Member not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn update_roles(
    State(harn): State<AppHarn>,
    Path(member_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<member_data::UpdateRolesData>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&member_id, &data.id)?;

    usecase::member::update_roles(harn.drive(), harn.repo(), user_token, data)
        .await?;

    no_content()
}

/// `DELETE /api/v1/members/{member_id}` — delete a member.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    delete,
    path = "/api/v1/members/{member_id}",
    tag = "members",
    params(("member_id" = String, Path, description = "Member ID")),
    responses(
        (status = 204, description = "Member deleted"),
        (status = 403, description = "No permission to delete this member"),
        (status = 404, description = "Member not found"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(member_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
) -> HttpNoContent {
    //
    usecase::member::delete(harn.drive(), harn.repo(), user_token, member_id)
        .await?;

    no_content()
}

/// `POST /api/v1/members/join` — join a team via invitation code.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/members/join",
    tag = "members",
    request_body = member_data::JoinTeamData,
    responses(
        (status = 201, description = "Joined team", body = HttpBody<member_data::InfoVal>),
        (status = 422, description = "Invitation does not target this user or already a member"),
        (status = 404, description = "Invitation code not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn join(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<member_data::JoinTeamData>,
) -> HttpResult<member_data::InfoVal> {
    usecase::member::join_team(
        harn.drive(),
        harn.repo(),
        harn.image_pool(),
        user_token,
        data,
    )
    .await?
    .accept(StatusCode::CREATED)
}
