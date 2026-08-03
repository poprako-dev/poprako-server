//! Member invitation handlers: create, list, role update, and deletion.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use serde::Deserialize;
use tracing::instrument;

use crate::data::instr::member_invitation::{
    CreateMemberInvitationInstr, ListMemberInvitationInfosInstr,
    UpdateMemberInvitationRolesInstr,
};
use crate::data::val::member_invitation::CreateMemberInvitationVal;
use crate::data::view::member_invitation::MemberInvitationInfoView;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;
#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::usecase;
use crate::value::member_invitation::MemberInvitationInclOpt;

/// Query for listing invitations within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?is_pending=true&incl=invitor&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct MemberInvitationListQuery {
    //
    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Related rows to embed. Repeatable. Values: `invitor`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<MemberInvitationInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/member-invitations` — create a pending team invitation.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/member-invitations",
    tag = "member-invitations",
    request_body = CreateMemberInvitationInstr,
    responses(
        (status = 201, description = "Invitation created", body = HttpBody<CreateMemberInvitationVal>),
        (status = 403, description = "No perm to create invitations in this team"),
        (status = 409, description = "Invitee is already a member"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateMemberInvitationInstr>,
) -> HttpResult<CreateMemberInvitationVal> {
    usecase::member_invitation::create(
        (harn.nucl(), harn.repo(), harn.prom()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/member-invitations` — list a team's invitations.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/member-invitations",
    tag = "member-invitations",
    description = "Lists a team's member invitations. `is_pending` filters by consumption state; `incl` embeds related rows. Example: `/api/v1/teams/{team_id}/member-invitations?is_pending=true&incl=invitor&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"), MemberInvitationListQuery),
    responses(
        (status = 200, description = "Invitations listed", body = HttpBody<Vec<MemberInvitationInfoView>>),
        (status = 403, description = "No perm to list invitations in this team"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<MemberInvitationListQuery>,
) -> HttpResult<Vec<MemberInvitationInfoView>> {
    //
    let instr = ListMemberInvitationInfosInstr {
        team_id,
        is_pending: query.is_pending,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::member_invitation::list_infos(
        (harn.repo(), harn.image_pool()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/member-invitations/{member_invitation_id}/roles` — update invitation roles.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/member-invitations/{member_invitation_id}/roles",
    tag = "member-invitations",
    params(("member_invitation_id" = String, Path, description = "Invitation ID")),
    request_body = UpdateMemberInvitationRolesInstr,
    responses(
        (status = 204, description = "Invitation roles updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No perm to update this invitation"),
        (status = 404, description = "Invitation not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_roles(
    State(harn): State<AppHarn>,
    Path(member_invitation_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateMemberInvitationRolesInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&member_invitation_id, &instr.id)?;

    usecase::member_invitation::update_roles(
        (harn.nucl(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/member-invitations/{member_invitation_id}` — delete an invitation.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/member-invitations/{member_invitation_id}",
    tag = "member-invitations",
    params(("member_invitation_id" = String, Path, description = "Invitation ID")),
    responses(
        (status = 204, description = "Invitation deleted"),
        (status = 403, description = "No perm to delete this invitation"),
        (status = 404, description = "Invitation not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(member_invitation_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::member_invitation::delete(
        (harn.nucl(), harn.repo()),
        user_token,
        member_invitation_id,
    )
    .await?;

    no_content()
}
