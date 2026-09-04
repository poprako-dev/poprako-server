//! Workset handlers: create, list, read, update, and delete.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::{
    Pagination, ensure_path_matches_body_id,
};

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::workset::{
    CreateWorksetInstr, ListWorksetInfosInstr, UpdateWorksetInfoInstr,
};
use crate::data::val::workset::CreateWorksetVal;
use crate::data::view::workset::WorksetInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;

/// `POST /api/v1/worksets` — create a workset inside a team.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/worksets",
    tag = "worksets",
    request_body = CreateWorksetInstr,
    responses(
        (status = 201, description = "Workset created", body = HttpBody<CreateWorksetVal>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "No perm to create worksets in this team"),
        (status = 404, description = "Team not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateWorksetInstr>,
) -> HttpResult<CreateWorksetVal> {
    //
    usecase::workset::create::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/worksets` — list worksets in a team.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/worksets",
    tag = "worksets",
    params(("team_id" = String, Path, description = "Team ID"), Pagination),
    responses(
        (status = 200, description = "Worksets listed", body = HttpBody<Vec<WorksetInfoView>>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "No perm to list worksets in this team"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(pagination): Query<Pagination>,
) -> HttpResult<Vec<WorksetInfoView>> {
    //
    let instr = ListWorksetInfosInstr {
        team_id,
        offset: pagination.offset,
        limit: pagination.limit,
    };

    usecase::workset::list_infos::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/worksets/{workset_id}` — fetch a workset by id.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/worksets/{workset_id}",
    tag = "worksets",
    params(("workset_id" = String, Path, description = "Workset ID")),
    responses(
        (status = 200, description = "Workset info retrieved", body = HttpBody<WorksetInfoView>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "No perm to view this workset"),
        (status = 404, description = "Workset not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<WorksetInfoView> {
    //
    usecase::workset::get_info::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        workset_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/worksets/{workset_id}` — update a workset's profile.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/worksets/{workset_id}",
    tag = "worksets",
    params(("workset_id" = String, Path, description = "Workset ID")),
    request_body = UpdateWorksetInfoInstr,
    responses(
        (status = 204, description = "Workset updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No perm to update this workset"),
        (status = 404, description = "Workset not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateWorksetInfoInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&workset_id, &instr.id)?;

    usecase::workset::update_info::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/worksets/{workset_id}` — delete a workset and descendants.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/worksets/{workset_id}",
    tag = "worksets",
    params(("workset_id" = String, Path, description = "Workset ID")),
    responses(
        (status = 204, description = "Workset deleted"),
        (status = 403, description = "No perm to delete this workset"),
        (status = 404, description = "Workset not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::workset::delete::<_, RdbContext<Serial>, HybRepo>(
        (harn.nucl().serial(), harn.repo()),
        user_token,
        workset_id,
    )
    .await?;

    no_content()
}
