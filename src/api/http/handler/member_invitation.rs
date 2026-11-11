//! Member invitation handlers: create, list, role update, and deletion.
use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::member_invitation::{
    CreateMemberInvitationBody, CreateMemberInvitationPayload,
    ListMemberInvitationInfosQuery, MemberInvitationInfoObj,
    MemberInvitationListQuery, UpdateMemberInvitationRolesBody,
};
use crate::api::http::result::{
    AcceptInto as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::prom::rdb_impl::writer::RdbProm;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::member_invitation as member_invitation_usecase;

/// `POST /api/v1/member-invitations` — create a pending team invitation.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/member-invitations",
    tag = "member-invitations",
    request_body = CreateMemberInvitationPayload,
    responses(
        (status = 201, description = "Invitation created",
            body = HttpBody<CreateMemberInvitationPayload>),
        (status = 403, description = "No perm to create invitations in this \
        team"),
        (status = 409, description = "Invitee is already a member"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateMemberInvitationBody>,
) -> HttpResult<CreateMemberInvitationPayload> {
    //
    Ok(member_invitation_usecase::create::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        RdbProm,
    >(
        (
            harn.nucl().rept_read(),
            harn.repo(),
            harn.prom(),
            &harn.config().invitation,
        ),
        user_token,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::CREATED))
}

/// `GET /api/v1/teams/{team_id}/member-invitations` — list a team's invitations.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/member-invitations",
    tag = "member-invitations",
    description = "Lists a team's member invitations. `is_pending` filters by \
    consumption state; `incl` embeds related rows. Example: \
    `/api/v1/teams/{team_id}/member-invitations?is_pending=true&\
    incl=invitor&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"),
        MemberInvitationListQuery),
    responses(
        (status = 200, description = "Invitations listed",
            body = HttpBody<Vec<MemberInvitationInfoObj>>),
        (status = 403, description = "No perm to list invitations in this \
        team"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<MemberInvitationListQuery>,
) -> HttpResult<Vec<MemberInvitationInfoObj>> {
    //
    let instr = ListMemberInvitationInfosQuery {
        team_id,
        is_pending: query.is_pending,
        incl: query.incl,
        offset: query.offset,
        limit: query.limit,
    };

    Ok(member_invitation_usecase::list_infos::<RdbContext<ReptRead>, RdbRepo, _>(
        (harn.repo(), harn.obj_dept()),
        user_token,
        instr.into(),
    )
    .await?.accept_into(StatusCode::OK))
}

/// `PUT /api/v1/member-invitations/{member_invitation_id}/roles` — update invitation roles.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/member-invitations/{member_invitation_id}/roles",
    tag = "member-invitations",
    params(("member_invitation_id" = String, Path,
        description = "Invitation ID")),
    request_body = UpdateMemberInvitationRolesBody,
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
    Json(instr): Json<UpdateMemberInvitationRolesBody>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&member_invitation_id, &instr.id)?;

    member_invitation_usecase::update_roles::<RdbContext<ReptRead>, RdbRepo>(
        (harn.repo(),),
        user_token,
        instr.into(),
    )
    .await?;

    Ok(no_content())
}

/// `DELETE /api/v1/member-invitations/{member_invitation_id}` — delete an invitation.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/member-invitations/{member_invitation_id}",
    tag = "member-invitations",
    params(("member_invitation_id" = String, Path,
        description = "Invitation ID")),
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
    member_invitation_usecase::delete::<RdbContext<ReptRead>, RdbRepo>(
        (harn.repo(),),
        user_token,
        member_invitation_id,
    )
    .await?;

    Ok(no_content())
}
