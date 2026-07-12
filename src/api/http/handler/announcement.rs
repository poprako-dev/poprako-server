//! Announcement handlers: create and list.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;

use serde::Deserialize;

use tracing::instrument;

#[cfg(feature = "swagger-ui")]
use utoipa::IntoParams;

#[cfg(feature = "swagger-ui")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{Accept as _, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::announcement_data;
use crate::model::user_model;
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
    pub offset: u64,

    /// Maximum number of items to return.
    pub limit: u64,
}

/// `POST /api/v1/announcements` — create a team announcement.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/announcements",
    tag = "announcements",
    request_body = announcement_data::CreateData,
    responses(
        (status = 201, description = "Announcement created", body = HttpBody<announcement_data::CreateVal>),
        (status = 403, description = "No permission to create announcements in this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<user_model::Token>,
    Json(data): Json<announcement_data::CreateData>,
) -> HttpResult<announcement_data::CreateVal> {
    usecase::announcement::create(harn.drive(), harn.repo(), user_token, data)
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
        (status = 200, description = "Announcements listed", body = HttpBody<Vec<announcement_data::InfoVal>>),
        (status = 403, description = "No permission to list announcements in this team"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<user_model::Token>,
    Query(query): Query<AnnouncementListQuery>,
) -> HttpResult<Vec<announcement_data::InfoVal>> {
    //
    let data = announcement_data::ListInfosData {
        team_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::announcement::list_infos(
        harn.repo(),
        harn.image_pool(),
        user_token,
        data,
    )
    .await?
    .accept(StatusCode::OK)
}
