//! Assignment invitation handlers: create, list, delete, and join.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;
#[cfg(feature = "swagger")]
use utoipa::IntoParams;

#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::assignment_invitation::{
    CreateAssignmentInvitationInstr, JoinAssignmentInvitationInstr,
    ListAssignmentInvitationInfosInstr,
};
use crate::data::val::assignment_invitation::CreateAssignmentInvitationVal;
use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::assignment_invitation::AssignmentInvitationInfoView;
use crate::model::shared::user::UserToken;
use crate::usecase;

/// Query for listing assignment invitations under one chapter.
///
/// Example: `?is_pending=true&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct AssignmentInvitationListQuery {
    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/assignment-invitations` — create a pending assignment invitation.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/assignment-invitations",
    tag = "assignment-invitations",
    request_body = CreateAssignmentInvitationInstr,
    responses(
        (status = 201, description = "Invitation created", body = HttpBody<CreateAssignmentInvitationVal>),
        (status = 403, description = "No perm to create invitations in this chapter"),
        (status = 409, description = "Invitee is already assigned"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateAssignmentInvitationInstr>,
) -> HttpResult<CreateAssignmentInvitationVal> {
    //
    usecase::assignment_invitation::create(
        (harn.nucl(), harn.repo(), harn.prom()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/chapters/{chapter_id}/assignment-invitations` — list invitations.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/assignment-invitations",
    tag = "assignment-invitations",
    description = "Lists a chapter's assignment invitations. `is_pending` filters by consumption state. Example: `/api/v1/chapters/{chapter_id}/assignment-invitations?is_pending=true&offset=0&limit=20`.",
    params(("chapter_id" = String, Path, description = "Chapter ID"), AssignmentInvitationListQuery),
    responses(
        (status = 200, description = "Invitations listed", body = HttpBody<Vec<AssignmentInvitationInfoView>>),
        (status = 403, description = "No perm to list invitations in this chapter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<AssignmentInvitationListQuery>,
) -> HttpResult<Vec<AssignmentInvitationInfoView>> {
    //
    let instr = ListAssignmentInvitationInfosInstr {
        chapter_id,
        is_pending: query.is_pending,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::assignment_invitation::list_infos(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `DELETE /api/v1/assignment-invitations/{assignment_invitation_id}` — delete.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/assignment-invitations/{assignment_invitation_id}",
    tag = "assignment-invitations",
    params(("assignment_invitation_id" = String, Path, description = "Invitation ID")),
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
    usecase::assignment_invitation::delete(
        (harn.nucl(), harn.repo()),
        user_token,
        assignment_invitation_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/assignment-invitations/join` — join via invitation code.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/assignment-invitations/join",
    tag = "assignment-invitations",
    request_body = JoinAssignmentInvitationInstr,
    responses(
        (status = 201, description = "Joined assignment", body = HttpBody<AssignmentInfoView>),
        (status = 422, description = "Invitation does not target this user"),
        (status = 403, description = "Role not assignable or no perm"),
        (status = 404, description = "Invitation code not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn join(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<JoinAssignmentInvitationInstr>,
) -> HttpResult<AssignmentInfoView> {
    //
    usecase::assignment_invitation::join(
        (harn.nucl(), harn.repo(), harn.image_pool()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}
