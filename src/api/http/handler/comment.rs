//! Comment handlers: create and list.
use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::comment::{
    CommentInfoObj, CommentListQuery, CreateCommentBody, CreateCommentPayload,
    ListCommentInfosQuery,
};
use crate::api::http::result::{AcceptInto as _, HttpResult};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::comment as comment_usecase;

/// `POST /api/v1/comments` — create a team board comment.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/comments",
    tag = "comments",
    request_body = CreateCommentBody,
    responses(
        (status = 201, description = "Comment \
        created", body = HttpBody<CreateCommentPayload>),
        (status = 403, description = "No perm to comment in this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateCommentBody>,
) -> HttpResult<CreateCommentPayload> {
    //
    Ok(comment_usecase::create::<RdbContext<ReptRead>, RdbRepo>(
        harn.repo(),
        user_token,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::CREATED))
}

/// `GET /api/v1/teams/{team_id}/comments` — list a team's comments.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/comments",
    tag = "comments",
    description = "Lists a team's board comments. `incl` embeds related rows. \
    Example: `/api/v1/teams/{team_id}/comments?incl=user&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"),
        CommentListQuery),
    responses(
        (status = 200, description = "Comments listed",
            body = HttpBody<Vec<CommentInfoObj>>),
        (status = 403, description = "No perm to list comments in this team"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<CommentListQuery>,
) -> HttpResult<Vec<CommentInfoObj>> {
    //
    let instr = ListCommentInfosQuery {
        team_id,
        incl: query.incl,
        offset: query.offset,
        limit: query.limit,
    };

    Ok(
        comment_usecase::list_infos::<RdbContext<ReptRead>, RdbRepo, _>(
            (harn.repo(), harn.obj_dept()),
            user_token,
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::OK),
    )
}
