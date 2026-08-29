//! Member handlers: create, join, list, role update, and deletion.

#[cfg(test)]
// Member handler tests validate request parameter shape and response mapping.
mod tests;

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use serde::Deserialize;
use tracing::instrument;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::member::{
    CreateMemberInstr, JoinTeamInstr, ListMemberInfosInstr,
    UpdateMemberRolesInstr,
};
use crate::data::val::member::CreateMemberVal;
use crate::data::view::member::MemberInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::member::MemberInclOpt;

/// Query for the current-user memberships list endpoint (`/members/me`).
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&incl=team&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct MemberMeListQuery {
    /// Related rows to embed. Repeatable. Values: `user`, `team`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<MemberInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/members` — create a member under a team.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/members",
    tag = "members",
    request_body = CreateMemberInstr,
    responses(
        (status = 201, description = "Member created", body = HttpBody<CreateMemberVal>),
        (status = 403, description = "No perm to create members in this team"),
        (status = 404, description = "User or team not found"),
        (status = 409, description = "User is already a member"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateMemberInstr>,
) -> HttpResult<CreateMemberVal> {
    //
    usecase::member::create::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/members` — list members by team or owner.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/members",
    tag = "members",
    description = "Lists members. Exactly one of `owner_id` or `team_id` is required. In `owner_id` mode, `role` and `fuzzy_nickname` must be omitted. In `team_id` mode, `fuzzy_nickname` and `role` are optional. `incl` embeds related rows. Examples: `/api/v1/members?team_id=t_1&fuzzy_nickname=al&role=1&incl=user`, `/api/v1/members?owner_id=u_1&incl=team`.",
    params(ListMemberInfosInstr),
    responses(
        (status = 200, description = "Members listed", body = HttpBody<Vec<MemberInfoView>>),
        (status = 422, description = "Exactly one of owner_id or team_id is required, or owner_id combined with role/fuzzy_nickname"),
        (status = 403, description = "No perm to list members in this team"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ListMemberInfosInstr>,
) -> HttpResult<Vec<MemberInfoView>> {
    //
    usecase::member::list_infos::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/members/me` — list the current user's memberships.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/members/me",
    tag = "members",
    params(MemberMeListQuery),
    responses(
        (status = 200, description = "Current user memberships", body = HttpBody<Vec<MemberInfoView>>),
        (status = 401, description = "Authentication required"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_my_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<MemberMeListQuery>,
) -> HttpResult<Vec<MemberInfoView>> {
    //
    let instr = ListMemberInfosInstr {
        owner_id: Some(user_token.user_id.clone()),
        team_id: None,
        fuzzy_nickname: None,
        role: None,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::member::list_infos::<RdbContext<ReptRead>, HybRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/members/{member_id}/roles` — update a member's roles.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/members/{member_id}/roles",
    tag = "members",
    params(("member_id" = String, Path, description = "Member ID")),
    request_body = UpdateMemberRolesInstr,
    responses(
        (status = 204, description = "Member roles updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No perm or the team would lose its last admin"),
        (status = 409, description = "Serializable conflict; retry the complete request"),
        (status = 404, description = "Member not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_roles(
    State(harn): State<AppHarn>,
    Path(member_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateMemberRolesInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&member_id, &instr.id)?;

    usecase::member::update_roles::<_, RdbContext<Serial>, HybRepo>(
        (harn.nucl().serial(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/members/{member_id}` — delete a member.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/members/{member_id}",
    tag = "members",
    params(("member_id" = String, Path, description = "Member ID")),
    responses(
        (status = 204, description = "Member deleted"),
        (status = 403, description = "No perm or the team would lose its last admin"),
        (status = 409, description = "Serializable conflict; retry the complete request"),
        (status = 404, description = "Member not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(member_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::member::delete::<_, RdbContext<Serial>, HybRepo>(
        (harn.nucl().serial(), harn.repo()),
        user_token,
        member_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/members/join` — join a team via invitation code.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/members/join",
    tag = "members",
    request_body = JoinTeamInstr,
    responses(
        (status = 201, description = "Joined team", body = HttpBody<MemberInfoView>),
        (status = 422, description = "Invitation does not target this user or already a member"),
        (status = 404, description = "Invitation code not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn join(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<JoinTeamInstr>,
) -> HttpResult<MemberInfoView> {
    //
    usecase::member::join_team::<_, RdbContext<ReptRead>, HybRepo, _>(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}
