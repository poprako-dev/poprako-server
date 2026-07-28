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
use crate::data::assignment::AssignmentInfoVal;
use crate::data::assignment_invitation::{
    AssignmentInvitationInfoVal, CreateAssignmentInvitationParams,
    CreateAssignmentInvitationPayload, JoinAssignmentInvitationParams,
    ListAssignmentInvitationInfosParams,
};
use crate::model::user::UserToken;
use crate::usecase;

/// Query for listing assignment invitations under one chapter.
///
/// Example: `?pending=true&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct AssignmentInvitationListQuery {
    //
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
    request_body = CreateAssignmentInvitationParams,
    responses(
        (status = 201, description = "Invitation created", body = HttpBody<CreateAssignmentInvitationPayload>),
        (status = 403, description = "No permission to create invitations in this chapter"),
        (status = 409, description = "Invitee is already assigned"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<CreateAssignmentInvitationParams>,
) -> HttpResult<CreateAssignmentInvitationPayload> {
    usecase::assignment_invitation::create(
        (harn.drive(), harn.repo(), harn.prom()),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/chapters/{chapter_id}/assignment-invitations` — list invitations.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/chapters/{chapter_id}/assignment-invitations",
    tag = "assignment-invitations",
    description = "Lists a chapter's assignment invitations. `pending` filters by consumption state. Example: `/api/v1/chapters/{chapter_id}/assignment-invitations?pending=true&offset=0&limit=20`.",
    params(("chapter_id" = String, Path, description = "Chapter ID"), AssignmentInvitationListQuery),
    responses(
        (status = 200, description = "Invitations listed", body = HttpBody<Vec<AssignmentInvitationInfoVal>>),
        (status = 403, description = "No permission to list invitations in this chapter"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<AssignmentInvitationListQuery>,
) -> HttpResult<Vec<AssignmentInvitationInfoVal>> {
    //
    let params = ListAssignmentInvitationInfosParams {
        chapter_id,
        is_pending: query.is_pending,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::assignment_invitation::list_infos(
        (harn.repo(),),
        user_token,
        params,
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
        (status = 403, description = "No permission to delete this invitation"),
        (status = 404, description = "Invitation not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(assignment_invitation_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::assignment_invitation::delete(
        (harn.drive(), harn.repo()),
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
    request_body = JoinAssignmentInvitationParams,
    responses(
        (status = 201, description = "Joined assignment", body = HttpBody<AssignmentInfoVal>),
        (status = 422, description = "Invitation does not target this user"),
        (status = 403, description = "Role not assignable or no permission"),
        (status = 404, description = "Invitation code not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn join(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<JoinAssignmentInvitationParams>,
) -> HttpResult<AssignmentInfoVal> {
    usecase::assignment_invitation::join(
        (harn.drive(), harn.repo(), harn.image_pool()),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::CREATED)
}
