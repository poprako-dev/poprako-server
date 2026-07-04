//! Comment handlers: create and list.

use axum::Json;
use axum::extract::Extension;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;

use serde::Deserialize;

use tracing::instrument;

use utoipa::IntoParams;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpResult;
use crate::api::http::state::AppHarn;
use crate::data::comment::{
    CommentInfoVal, CreateCommentData, CreateCommentVal, ListCommentInfosData,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::comment::CommentInclOpt;

/// Query for listing comments within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CommentListQuery {
    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<CommentInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u64,

    /// Maximum number of items to return.
    pub limit: u64,
}

/// `POST /api/v1/comments` — create a team board comment.
#[utoipa::path(
    post,
    path = "/api/v1/comments",
    tag = "comments",
    request_body = CreateCommentData,
    responses(
        (status = 201, description = "Comment created", body = CreateCommentVal),
        (status = 403, description = "No permission to comment in this team"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<CreateCommentData>,
) -> HttpResult<CreateCommentVal> {
    let reply = usecase::comment::create(harn.drive(), harn.repo(), user_token, data).await?;
    reply.accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/comments` — list a team's comments.
#[utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/comments",
    tag = "comments",
    description = "Lists a team's board comments. `incl` embeds related rows. Example: `/api/v1/teams/{team_id}/comments?incl=user&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"), CommentListQuery),
    responses(
        (status = 200, description = "Comments listed", body = Vec<CommentInfoVal>),
        (status = 403, description = "No permission to list comments in this team"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<CommentListQuery>,
) -> HttpResult<Vec<CommentInfoVal>> {
    let data = ListCommentInfosData {
        team_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    let infos =
        usecase::comment::list_infos(harn.repo(), harn.image_pool(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}
