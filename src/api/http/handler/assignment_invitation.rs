//! Assignment invitation handlers: create, list, delete, and join.
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::assignment::AssignmentInfoObj;
use crate::api::http::object::assignment_invitation::{
    AssignmentInvitationInfoObj, AssignmentInvitationListQuery,
    CreateAssignmentInvitationBody, CreateAssignmentInvitationPayload,
    JoinAssignmentInvitationBody, ListAssignmentInvitationInfosQuery,
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
use crate::usecase::assignment_invitation as assignment_invitation_usecase;
use crate::usecase::assignment_invitation::join as assignment_invitation_join_usecase;

/// `POST /api/v1/assignment-invitations` — create a pending assignment invitation.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/assignment-invitations",
    tag = "assignment-invitations",
    description = "Requires owning-team administrator membership, without a \
    chapter assignment. Invitations contain worker roles only.",
    request_body = CreateAssignmentInvitationBody,
    responses(
        (status = 201, description = "Invitation created",
            body = HttpBody<CreateAssignmentInvitationPayload>),
        (status = 403, description = "No perm to create invitations in this \
        chapter"),
        (status = 409, description = "Invitee is already assigned"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateAssignmentInvitationBody>,
) -> HttpResult<CreateAssignmentInvitationPayload> {
    //
    Ok(assignment_invitation_usecase::create::<
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

/// `GET /api/v1/chapters/{chapter_id}/assignment-invitations` — list invitations.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/assignment-invitations",
    tag = "assignment-invitations",
    description = "Lists a chapter's assignment invitations. Requires \
    owning-team administrator membership. `is_pending` filters by consumption \
    state. Example: \
    `/api/v1/chapters/{chapter_id}/assignment-invitations?is_pending=true&\
    offset=0&limit=20`.",
    params(("chapter_id" = String, Path, description = "Chapter ID"),
        AssignmentInvitationListQuery),
    responses(
        (status = 200, description = "Invitations listed",
            body = HttpBody<Vec<AssignmentInvitationInfoObj>>),
        (status = 403, description = "No perm to list invitations in this \
        chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<AssignmentInvitationListQuery>,
) -> HttpResult<Vec<AssignmentInvitationInfoObj>> {
    //
    let instr = ListAssignmentInvitationInfosQuery {
        chapter_id,
        is_pending: query.is_pending,
        offset: query.offset,
        limit: query.limit,
    };

    Ok(
        assignment_invitation_usecase::list_infos::<
            RdbContext<ReptRead>,
            RdbRepo,
        >((harn.repo(),), user_token, instr.into())
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// `DELETE /api/v1/assignment-invitations/{assignment_invitation_id}` — delete.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/assignment-invitations/{assignment_invitation_id}",
    tag = "assignment-invitations",
    params(("assignment_invitation_id" = String, Path,
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
    Path(assignment_invitation_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    assignment_invitation_usecase::delete::<_, RdbContext<ReptRead>, RdbRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        assignment_invitation_id,
    )
    .await?;

    Ok(no_content())
}

/// `POST /api/v1/assignment-invitations/join` — join via invitation code.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/assignment-invitations/join",
    tag = "assignment-invitations",
    request_body = JoinAssignmentInvitationBody,
    responses(
        (status = 201, description = "Joined \
        assignment", body = HttpBody<AssignmentInfoObj>),
        (status = 422, description = "Invitation does not target this user"),
        (status = 403, description = "Role not assignable or no perm"),
        (status = 404, description = "Invitation code not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn join(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<JoinAssignmentInvitationBody>,
) -> HttpResult<AssignmentInfoObj> {
    //
    Ok(assignment_invitation_join_usecase::join::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
    >(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        user_token,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::CREATED))
}
