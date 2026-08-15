//! Announcement handlers: create and list.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use serde::Deserialize;
use tracing::instrument;
#[cfg(feature = "swagger")]
use utoipa::IntoParams;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;
use crate::api::http::result::{Accept as _, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::instr::announcement::{
    CreateAnnouncementInstr, ListAnnouncementInfosInstr,
};
use crate::data::val::announcement::CreateAnnouncementVal;
use crate::data::view::announcement::AnnouncementInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::RepeatableRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::announcement::AnnouncementInclOpt;

/// Query for listing announcements within a team.
///
/// `incl` embeds related rows into each item.
///
/// Example: `?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct AnnouncementListQuery {
    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<AnnouncementInclOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/announcements` — create a team announcement.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/announcements",
    tag = "announcements",
    request_body = CreateAnnouncementInstr,
    responses(
        (status = 201, description = "Announcement created", body = HttpBody<CreateAnnouncementVal>),
        (status = 403, description = "No perm to create announcements in this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateAnnouncementInstr>,
) -> HttpResult<CreateAnnouncementVal> {
    //
    usecase::announcement::create::<_, RdbContext<RepeatableRead>, HybRepo>(
        (harn.nucl().repeatable_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/announcements` — list a team's announcements.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/announcements",
    tag = "announcements",
    description = "Lists a team's announcements. `incl` embeds related rows. Example: `/api/v1/teams/{team_id}/announcements?incl=user&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"), AnnouncementListQuery),
    responses(
        (status = 200, description = "Announcements listed", body = HttpBody<Vec<AnnouncementInfoView>>),
        (status = 403, description = "No perm to list announcements in this team"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<AnnouncementListQuery>,
) -> HttpResult<Vec<AnnouncementInfoView>> {
    //
    let instr = ListAnnouncementInfosInstr {
        team_id,
        incl_opt: query.incl_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::announcement::list_infos::<RdbContext<RepeatableRead>, HybRepo, _>(
        (harn.repo(), harn.image_pool()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}
