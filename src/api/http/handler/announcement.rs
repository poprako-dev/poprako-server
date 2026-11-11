//! Announcement handlers.
use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::announcement::{
    AnnouncementInfoObj, AnnouncementListQuery, CreateAnnouncementBody,
    CreateAnnouncementPayload, ListAnnouncementInfosQuery,
    UpdateAnnouncementInfoBody,
};
use crate::api::http::result::{
    AcceptInto as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::announcement as announcement_usecase;

/// `POST /api/v1/announcements` — create a team announcement.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/announcements",
    tag = "announcements",
    request_body = CreateAnnouncementPayload,
    responses(
        (status = 201, description = "Announcement created",
            body = HttpBody<CreateAnnouncementPayload>),
        (status = 403, description = "No perm to create announcements in \
        this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateAnnouncementBody>,
) -> HttpResult<CreateAnnouncementPayload> {
    //
    Ok(
        announcement_usecase::create::<RdbContext<ReptRead>, RdbRepo>(
            harn.repo(),
            user_token,
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::CREATED),
    )
}

/// `GET /api/v1/teams/{team_id}/announcements` — list a team's announcements.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/announcements",
    tag = "announcements",
    description = "Lists a team's announcements. `incl` embeds related rows. \
    Example: \
    `/api/v1/teams/{team_id}/announcements?incl=user&offset=0&limit=20`.",
    params(("team_id" = String, Path, description = "Team ID"),
        AnnouncementListQuery),
    responses(
        (status = 200, description = "Announcements listed",
            body = HttpBody<Vec<AnnouncementInfoObj>>),
        (status = 403, description = "No perm to list announcements in this \
        team"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<AnnouncementListQuery>,
) -> HttpResult<Vec<AnnouncementInfoObj>> {
    //
    let instr = ListAnnouncementInfosQuery {
        team_id,
        incl: query.incl,
        offset: query.offset,
        limit: query.limit,
    };

    Ok(
        announcement_usecase::list_infos::<RdbContext<ReptRead>, RdbRepo, _>(
            (harn.repo(), harn.obj_dept()),
            user_token,
            instr.into(),
        )
        .await?
        .accept_into(StatusCode::OK),
    )
}

/// `PUT /api/v1/announcements/{announcement_id}` — replace editable fields.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/announcements/{announcement_id}",
    tag = "announcements",
    params(("announcement_id" = String, Path, description = "Announcement ID")),
    request_body = UpdateAnnouncementInfoBody,
    responses(
        (status = 204, description = "Announcement updated"),
        (status = 403, description = "Team admin role required"),
        (status = 422, description = "Announcement not found or path mismatch"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(announcement_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateAnnouncementInfoBody>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&announcement_id, &instr.id)?;

    announcement_usecase::update_info::<RdbContext<ReptRead>, RdbRepo>(
        harn.repo(),
        user_token,
        instr.into(),
    )
    .await?;

    Ok(no_content())
}

/// `DELETE /api/v1/announcements/{announcement_id}` — delete an announcement.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/announcements/{announcement_id}",
    tag = "announcements",
    params(("announcement_id" = String, Path, description = "Announcement ID")),
    responses(
        (status = 204, description = "Announcement deleted"),
        (status = 403, description = "Team admin role required"),
        (status = 422, description = "Announcement not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(announcement_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    announcement_usecase::delete::<RdbContext<ReptRead>, RdbRepo>(
        harn.repo(),
        user_token,
        announcement_id,
    )
    .await?;

    Ok(no_content())
}
