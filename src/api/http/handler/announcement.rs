//! Announcement handlers: create and list.

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
use crate::data::announcement::{
    AnnouncementInfoVal, CreateAnnouncementData, CreateAnnouncementVal, ListAnnouncementInfosData,
};
use crate::model::user::UserToken;
use crate::usecase;
use crate::value::announcement::AnnouncementInclOpt;

/// Query for listing announcements within a team.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AnnouncementListQuery {
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<AnnouncementInclOpt>,

    pub offset: u64,
    pub limit: u64,
}

/// `POST /api/v1/announcements` — create a team announcement.
#[utoipa::path(
    post,
    path = "/api/v1/announcements",
    tag = "announcements",
    request_body = CreateAnnouncementData,
    responses(
        (status = 201, description = "Announcement created", body = CreateAnnouncementVal),
        (status = 403, description = "No permission to create announcements in this team"),
        (status = 404, description = "Team not found"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(data): Json<CreateAnnouncementData>,
) -> HttpResult<CreateAnnouncementVal> {
    let reply = usecase::announcement::create(harn.drive(), harn.repo(), user_token, data).await?;

    reply.accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/announcements` — list a team's announcements.
#[utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/announcements",
    tag = "announcements",
    params(("team_id" = String, Path, description = "Team ID"), AnnouncementListQuery),
    responses(
        (status = 200, description = "Announcements listed", body = Vec<AnnouncementInfoVal>),
        (status = 403, description = "No permission to list announcements in this team"),
    ),
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<AnnouncementListQuery>,
) -> HttpResult<Vec<AnnouncementInfoVal>> {
    let data = ListAnnouncementInfosData {
        team_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    let infos =
        usecase::announcement::list_infos(harn.repo(), harn.image_pool(), user_token, data).await?;

    infos.accept(StatusCode::OK)
}
