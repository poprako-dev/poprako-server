//! Assignment handlers: list, join, role update, and deletion.

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
use crate::data::assignment::AssignmentInfoVal;
use crate::data::assignment::JoinChapterAssignmentParams;
use crate::data::assignment::ListAssignmentInfosParams;
use crate::data::assignment::UpdateAssignmentRolesParams;
use crate::model::user::UserToken;
use crate::usecase;

/// `GET /api/v1/assignments` — list assignments by chapter or owner.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/assignments",
    tag = "assignments",
    description = "Lists assignments. Exactly one of `chapter_id` or `owner_id` is required; `role` optionally narrows by a single role bit. `incl` embeds related rows; dotted values imply their parent segments. Examples: `/api/v1/assignments?chapter_id=c_1&role=1&incl=chapter.comic.workset.team`, `/api/v1/assignments?owner_id=u_1&incl=user`.",
    params(ListAssignmentInfosParams),
    responses(
        (status = 200, description = "Assignments listed", body = HttpBody<Vec<AssignmentInfoVal>>),
        (status = 422, description = "Exactly one of chapter_id or owner_id is required"),
        (status = 403, description = "No permission to list these assignments"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<ListAssignmentInfosParams>,
) -> HttpResult<Vec<AssignmentInfoVal>> {
    usecase::assignment::list_infos(
        harn.repo(),
        harn.image_pool(),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/chapters/{chapter_id}/assignments/{user_id}/roles` — update roles.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    put,
    path = "/api/v1/chapters/{chapter_id}/assignments/{user_id}/roles",
    tag = "assignments",
    params(
        ("chapter_id" = String, Path, description = "Chapter ID"),
        ("user_id" = String, Path, description = "Assignee user ID"),
    ),
    request_body = UpdateAssignmentRolesParams,
    responses(
        (status = 204, description = "Assignment roles updated"),
        (status = 422, description = "Path ids do not match body ids"),
        (status = 403, description = "No permission to update this assignment"),
        (status = 404, description = "Assignment not found"),
    ),
))]
#[instrument(err, skip(harn, params))]
pub async fn update_roles(
    State(harn): State<AppHarn>,
    Path((chapter_id, user_id)): Path<(String, String)>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<UpdateAssignmentRolesParams>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&chapter_id, &params.chapter_id)?;

    ensure_path_matches_body_id(&user_id, &params.user_id)?;

    usecase::assignment::update_roles(
        harn.drive(),
        harn.repo(),
        user_token,
        params,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/assignments/{assignment_id}` — delete an assignment.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    delete,
    path = "/api/v1/assignments/{assignment_id}",
    tag = "assignments",
    params(("assignment_id" = String, Path, description = "Assignment ID")),
    responses(
        (status = 204, description = "Assignment deleted"),
        (status = 403, description = "No permission to delete this assignment"),
        (status = 404, description = "Assignment not found"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(assignment_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::assignment::delete(
        harn.drive(),
        harn.repo(),
        user_token,
        assignment_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/assignments/join` — join a chapter assignment with roles.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/assignments/join",
    tag = "assignments",
    request_body = JoinChapterAssignmentParams,
    responses(
        (status = 201, description = "Joined assignment", body = HttpBody<AssignmentInfoVal>),
        (status = 403, description = "Role not assignable or no permission"),
        (status = 404, description = "Chapter not found"),
    ),
))]
#[instrument(err, skip(harn, params))]
pub async fn join(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<JoinChapterAssignmentParams>,
) -> HttpResult<AssignmentInfoVal> {
    usecase::assignment::join(harn.drive(), harn.repo(), user_token, params)
        .await?
        .accept(StatusCode::CREATED)
}
