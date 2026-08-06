//! Terminology-base handlers.

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;

use crate::data::instr::termbase::{
    CreateTermbaseInstr, ListComicTermbaseInfosInstr,
    ListTeamTermbaseInfosInstr, UpdateTermbaseInfoInstr,
};
use crate::data::val::termbase::CreateTermbaseVal;
use crate::data::view::termbase::TermbaseInfoView;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;
#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::usecase;

/// Query parameters for terminology-base lists.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct TermbaseListQuery {
    /// Optional case-insensitive name substring.
    pub fuzzy_name: Option<String>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}

/// `POST /api/v1/termbases` — create a terminology base.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/termbases",
    tag = "termbases",
    request_body = CreateTermbaseInstr,
    responses(
        (status = 201, description = "Termbase created", body = HttpBody<CreateTermbaseVal>),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Invalid scope, parent, or duplicate name"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateTermbaseInstr>,
) -> HttpResult<CreateTermbaseVal> {
    //
    usecase::termbase::create((harn.nucl(), harn.repo()), user_token, instr)
        .await?
        .accept(StatusCode::CREATED)
}

/// `GET /api/v1/teams/{team_id}/termbases` — list team terminology bases.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/termbases",
    tag = "termbases",
    params(("team_id" = String, Path, description = "Team ID"), TermbaseListQuery),
    responses(
        (status = 200, description = "Team termbases listed", body = HttpBody<Vec<TermbaseInfoView>>),
        (status = 403, description = "Team membership required"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_team_infos(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TermbaseListQuery>,
) -> HttpResult<Vec<TermbaseInfoView>> {
    //
    let instr = ListTeamTermbaseInfosInstr {
        team_id,
        fuzzy_name: query.fuzzy_name,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::termbase::list_team_infos((harn.repo(),), user_token, instr)
        .await?
        .accept(StatusCode::OK)
}

/// `GET /api/v1/comics/{comic_id}/termbases` — list visible terminology bases.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}/termbases",
    tag = "termbases",
    params(("comic_id" = String, Path, description = "Comic ID"), TermbaseListQuery),
    responses(
        (status = 200, description = "Comic-visible termbases listed", body = HttpBody<Vec<TermbaseInfoView>>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_comic_infos(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TermbaseListQuery>,
) -> HttpResult<Vec<TermbaseInfoView>> {
    //
    let instr = ListComicTermbaseInfosInstr {
        comic_id,
        fuzzy_name: query.fuzzy_name,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::termbase::list_comic_infos((harn.repo(),), user_token, instr)
        .await?
        .accept(StatusCode::OK)
}

/// `GET /api/v1/termbases/{termbase_id}` — fetch a terminology base.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}",
    tag = "termbases",
    params(("termbase_id" = String, Path, description = "Termbase ID")),
    responses(
        (status = 200, description = "Termbase retrieved", body = HttpBody<TermbaseInfoView>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<TermbaseInfoView> {
    //
    usecase::termbase::get_info((harn.repo(),), user_token, termbase_id)
        .await?
        .accept(StatusCode::OK)
}

/// `PUT /api/v1/termbases/{termbase_id}` — replace editable fields.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/termbases/{termbase_id}",
    tag = "termbases",
    params(("termbase_id" = String, Path, description = "Termbase ID")),
    request_body = UpdateTermbaseInfoInstr,
    responses(
        (status = 204, description = "Termbase updated"),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Invalid input, duplicate name, or path mismatch"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateTermbaseInfoInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&termbase_id, &instr.id)?;

    usecase::termbase::update_info(
        (harn.nucl(), harn.repo()),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `DELETE /api/v1/termbases/{termbase_id}` — delete a terminology base.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/termbases/{termbase_id}",
    tag = "termbases",
    params(("termbase_id" = String, Path, description = "Termbase ID")),
    responses(
        (status = 204, description = "Termbase and terms deleted"),
        (status = 403, description = "Team proofreader role required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::termbase::delete(
        (harn.nucl(), harn.repo()),
        user_token,
        termbase_id,
    )
    .await?;

    no_content()
}
