//! Member handlers: create, join, list, role update, and deletion.

use axum::Json;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;

use serde::Deserialize;

use tracing::instrument;

use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;
use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpNoContent;
use crate::api::http::result::HttpResult;
use crate::api::http::result::no_content;
use crate::api::http::state::AppHarn;
use crate::data::member::{
    CreateMemberData, CreateMemberVal, JoinTeamData, ListMemberInfosData, MemberInfoVal,
    UpdateMemberRolesData,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::member::MemberInclOpt;

/// Query for the current-user memberships list endpoint (`/members/me`).
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&incl=team&offset=0&limit=20`.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberMeListQuery {
    /// Related rows to embed. Repeatable. Values: `user`, `team`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<MemberInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u64,

    /// Maximum number of items to return.
    pub limit: u64,
}

/// `POST /api/v1/members` — create a member under a team.
#[utoipa::path(
    post,
    path = "/api/v1/members",
    tag = "members",
    request_body = CreateMemberData,
    responses(
        (status = 201, description = "Member created", body = CreateMemberVal),
        (status = 403, description = "No permission to create members in this team"),
        (status = 404, description = "User or team not found"),
        (status = 409, description = "User is already a member"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<CreateMemberData>,
) -> HttpResult<CreateMemberVal> {
    let reply = usecase::member::create(harn.drive(), harn.repo(), user_token, data).await?;
    reply.accept(StatusCode::CREATED)
}

/// `GET /api/v1/members` — list members by team or owner.
#[utoipa::path(
    get,
    path = "/api/v1/members",
    tag = "members",
    description = "Lists members. Exactly one of `owner_id` or `team_id` is required. In `owner_id` mode, `role` and `fuzzy_nickname` must be omitted. In `team_id` mode, `fuzzy_nickname` and `role` are optional. `incl` embeds related rows. Examples: `/api/v1/members?team_id=t_1&fuzzy_nickname=al&role=1&incl=user`, `/api/v1/members?owner_id=u_1&incl=team`.",
    params(ListMemberInfosData),
    responses(
        (status = 200, description = "Members listed", body = Vec<MemberInfoVal>),
        (status = 400, description = "Exactly one of owner_id or team_id is required, or owner_id combined with role/fuzzy_nickname"),
        (status = 403, description = "No permission to list members in this team"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(data): Query<ListMemberInfosData>,
) -> HttpResult<Vec<MemberInfoVal>> {
    let infos =
        usecase::member::list_infos(harn.repo(), harn.image_pool(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}

/// `GET /api/v1/members/me` — list the current user's memberships.
#[utoipa::path(
    get,
    path = "/api/v1/members/me",
    tag = "members",
    params(MemberMeListQuery),
    responses(
        (status = 200, description = "Current user memberships", body = Vec<MemberInfoVal>),
        (status = 401, description = "Authentication required"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_my_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<MemberMeListQuery>,
) -> HttpResult<Vec<MemberInfoVal>> {
    let data = ListMemberInfosData {
        owner_id: Some(user_token.user_id.clone()),
        team_id: None,
        fuzzy_nickname: None,
        role: None,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    let infos =
        usecase::member::list_infos(harn.repo(), harn.image_pool(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}

/// `PUT /api/v1/members/{member_id}/roles` — update a member's roles.
#[utoipa::path(
    put,
    path = "/api/v1/members/{member_id}/roles",
    tag = "members",
    params(("member_id" = String, Path, description = "Member ID")),
    request_body = UpdateMemberRolesData,
    responses(
        (status = 204, description = "Member roles updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No permission to update this member"),
        (status = 404, description = "Member not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn update_roles(
    State(harn): State<AppHarn>,
    Path(member_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<UpdateMemberRolesData>,
) -> HttpNoContent {
    ensure_path_matches_body_id(&member_id, &data.id)?;

    usecase::member::update_roles(harn.drive(), harn.repo(), user_token, data).await?;

    no_content()
}

/// `DELETE /api/v1/members/{member_id}` — delete a member.
#[utoipa::path(
    delete,
    path = "/api/v1/members/{member_id}",
    tag = "members",
    params(("member_id" = String, Path, description = "Member ID")),
    responses(
        (status = 204, description = "Member deleted"),
        (status = 403, description = "No permission to delete this member"),
        (status = 404, description = "Member not found"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(member_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    usecase::member::delete(harn.drive(), harn.repo(), user_token, member_id).await?;
    no_content()
}

/// `POST /api/v1/members/join` — join a team via invitation code.
#[utoipa::path(
    post,
    path = "/api/v1/members/join",
    tag = "members",
    request_body = JoinTeamData,
    responses(
        (status = 201, description = "Joined team", body = MemberInfoVal),
        (status = 400, description = "Invitation does not target this user or already a member"),
        (status = 404, description = "Invitation code not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn join(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<JoinTeamData>,
) -> HttpResult<MemberInfoVal> {
    let reply = usecase::member::join_team(
        harn.drive(),
        harn.repo(),
        harn.image_pool(),
        user_token,
        data,
    )
    .await?;
    reply.accept(StatusCode::CREATED)
}
