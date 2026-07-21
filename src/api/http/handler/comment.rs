//! Comment handlers: create and list.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;

#[cfg(feature = "swagger-ui")]
use utoipa::IntoParams;

#[allow(unused_imports)]
use crate::api::http::result::{Accept as _, HttpBody, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::comment::{
    CommentInfoVal, CreateCommentParams, CreateCommentPayload,
    ListCommentInfosParams,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::comment::CommentInclOpt;
use crate::value::query::GroupedQuery;

/// Query for listing comments within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct CommentListQuery {
    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<CommentInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/comments` — create a team board comment.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/comments",
    tag = "comments",
    request_body = CreateCommentParams,
    responses(
        (status = 201, description = "Comment created", body = HttpBody<CreateCommentPayload>),
        (status = 403, description = "No permission to comment in this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<CreateCommentParams>,
) -> HttpResult<CreateCommentPayload> {
    usecase::comment::create(harn.drive(), harn.repo(), user_token, params)
        .await?
        .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/comments` — list a team's comments.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/comments",
    tag = "comments",
    description = "Lists a team's board comments. `incl` embeds related rows. Example: `/api/v1/teams/{team_id}/comments?incl=user&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"), CommentListQuery),
    responses(
        (status = 200, description = "Comments listed", body = HttpBody<Vec<CommentInfoVal>>),
        (status = 403, description = "No permission to list comments in this team"),
    ),
))]
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    GroupedQuery(query): GroupedQuery<CommentListQuery>,
) -> HttpResult<Vec<CommentInfoVal>> {
    //
    let params = ListCommentInfosParams {
        team_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::comment::list_infos(
        harn.repo(),
        harn.image_pool(),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}
