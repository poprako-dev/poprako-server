//! Announcement handlers: create and list.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;

use serde::Deserialize;

use tracing::instrument;

#[cfg(feature = "swagger-ui")]
use utoipa::IntoParams;

#[allow(unused_imports)]
use crate::api::http::result::{Accept as _, HttpBody, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::announcement::AnnouncementInfoVal;
use crate::data::announcement::CreateAnnouncementParams;
use crate::data::announcement::CreateAnnouncementPayload;
use crate::data::announcement::ListAnnouncementInfosParams;
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::announcement::AnnouncementInclOpt;

/// Query for listing announcements within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct AnnouncementListQuery {
    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<AnnouncementInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/announcements` — create a team announcement.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/announcements",
    tag = "announcements",
    request_body = CreateAnnouncementParams,
    responses(
        (status = 201, description = "Announcement created", body = HttpBody<CreateAnnouncementPayload>),
        (status = 403, description = "No permission to create announcements in this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(err, skip(harn, params))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<CreateAnnouncementParams>,
) -> HttpResult<CreateAnnouncementPayload> {
    usecase::announcement::create(harn.drive(), harn.repo(), user_token, params)
        .await?
        .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/announcements` — list a team's announcements.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/announcements",
    tag = "announcements",
    description = "Lists a team's announcements. `incl` embeds related rows. Example: `/api/v1/teams/{team_id}/announcements?incl=user&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"), AnnouncementListQuery),
    responses(
        (status = 200, description = "Announcements listed", body = HttpBody<Vec<AnnouncementInfoVal>>),
        (status = 403, description = "No permission to list announcements in this team"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<AnnouncementListQuery>,
) -> HttpResult<Vec<AnnouncementInfoVal>> {
    //
    let params = ListAnnouncementInfosParams {
        team_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::announcement::list_infos(
        harn.repo(),
        harn.image_pool(),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}
