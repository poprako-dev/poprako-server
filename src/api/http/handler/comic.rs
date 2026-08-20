//! Comic handlers: CRUD, cover upload flow, and immutable archiving.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum_extra::extract::Query;
use serde::Deserialize;
use tracing::instrument;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::comic::{
    CreateComicInstr, ListComicInfosInstr, MarkComicCoverUploadedInstr,
    ReserveComicCoverInstr, UpdateComicInfoInstr,
};
use crate::data::instr::comic_archive::ExportComicArchivesInstr;
use crate::data::val::comic::{CreateComicVal, ReserveComicCoverVal};
use crate::data::val::comic_archive::{
    ArchiveComicVal, ExportComicArchivesVal,
};
use crate::data::val::comic_list::ListComicInfosVal;
use crate::data::view::comic::ComicInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::{RepeatableRead, Serializable};
use crate::part_impl::prom::rdb_impl::RdbProm;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;
use crate::value::comic::{ComicInclOpt, ComicStatus, ComicWithOpt};

/// `GET /api/v1/teams/{team_id}/comic-archives/export` — export archive month slots.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/teams/{team_id}/comic-archives/export",
    tag = "comics",
    params(
        ("team_id" = String, Path, description = "Team ID"),
        ExportComicArchivesInstr,
    ),
    responses(
        (status = 200, description = "Archive JSON strings grouped by UTC month", body = HttpBody<ExportComicArchivesVal>),
        (status = 403, description = "No perm to export this team's archives"),
        (status = 422, description = "Invalid month selection"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn export_archives(
    State(harn): State<AppHarn>,
    Path(team_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(instr): Query<ExportComicArchivesInstr>,
) -> HttpResult<ExportComicArchivesVal> {
    //
    usecase::comic_archive::export::<RdbContext<RepeatableRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        team_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// Query for listing comics within a workset.
///
/// When present, `stages` narrows the list by pinned chapter workflow state.
///
/// `incl` embeds related rows into each item; `with` populates derived rows
/// in the parallel list payload.
/// Dotted `incl` values implicitly pull in their parent segments.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ComicListQuery {
    /// Fuzzy title substring filter (case-insensitive).
    pub fuzzy_title: Option<String>,

    /// Workflow stage bitmask filter for pinned chapters.
    pub stages: Option<u32>,

    /// Lifecycle state filter. Omit to list both active and archived comics.
    pub status: Option<ComicStatus>,

    /// Related rows to embed. Repeatable. Values: `workset`, `workset.team`,
    /// `creator`. Dotted values imply their parent segments.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<ComicInclOpt>,

    /// Derived rows to attach. Repeatable. Values: `pinned_chapter`,
    /// `pinned_chapter_assignment`. The assignment option requires the chapter
    /// option.
    #[serde(default, rename = "with")]
    pub with_opt: Vec<ComicWithOpt>,

    /// Pagination offset (0-based).
    pub offset: u32,

    /// Maximum number of items to return.
    pub limit: u32,
}

/// `POST /api/v1/comics` — create a comic (and its first chapter).
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/comics",
    tag = "comics",
    request_body = CreateComicInstr,
    responses(
        (status = 201, description = "Comic created", body = HttpBody<CreateComicVal>),
        (status = 403, description = "No perm to create comics in this workset"),
        (status = 404, description = "Workset not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateComicInstr>,
) -> HttpResult<CreateComicVal> {
    //
    usecase::comic::create::<_, RdbContext<RepeatableRead>, HybRepo>(
        (harn.nucl().repeatable_read(), harn.repo()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `GET /api/v1/worksets/{workset_id}/comics` — list comics in a workset.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/worksets/{workset_id}/comics",
    tag = "comics",
    description = "Lists active and archived comics in a workset with optional title, lifecycle, and workflow-stage filters. `incl` embeds related rows and `with` populates parallel derived rows.",
    params(("workset_id" = String, Path, description = "Workset ID"), ComicListQuery),
    responses(
        (status = 200, description = "Comics listed", body = HttpBody<ListComicInfosVal>),
        (status = 403, description = "No perm to list comics in this workset"),
        (status = 422, description = "Invalid query option combination or workflow-stage filter"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(workset_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<ComicListQuery>,
) -> HttpResult<ListComicInfosVal> {
    //
    let instr = ListComicInfosInstr {
        workset_id,
        fuzzy_title: query.fuzzy_title,
        stages: query.stages,
        status: query.status,
        incl_opt: query.incl_opt,
        with_opt: query.with_opt,
        offset: query.offset,
        limit: query.limit,
    };

    usecase::comic::list::list_infos::<RdbContext<RepeatableRead>, HybRepo, _>(
        (harn.repo(), harn.image_pool()),
        user_token,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/comics/{comic_id}` — fetch a comic by id.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 200, description = "Comic info retrieved", body = HttpBody<ComicInfoView>),
        (status = 403, description = "No perm to view this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ComicInfoView> {
    //
    usecase::comic::get_info::<RdbContext<RepeatableRead>, HybRepo, _>(
        (harn.repo(), harn.image_pool()),
        user_token,
        comic_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/comics/{comic_id}` — update a comic's profile.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = UpdateComicInfoInstr,
    responses(
        (status = 204, description = "Comic updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "No perm to update this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateComicInfoInstr>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&comic_id, &instr.id)?;

    usecase::comic::update_info::<RdbContext<RepeatableRead>, HybRepo>(
        (harn.repo(),),
        user_token,
        instr,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/comics/{comic_id}/cover/reserve` — reserve a cover upload slot.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/cover/reserve",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = ReserveComicCoverInstr,
    responses(
        (status = 200, description = "Cover upload URL reserved", body = HttpBody<ReserveComicCoverVal>),
        (status = 403, description = "No perm to modify this comic's cover"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn reserve_cover(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<ReserveComicCoverInstr>,
) -> HttpResult<ReserveComicCoverVal> {
    //
    usecase::comic::reserve::reserve_cover::<
        _,
        RdbContext<RepeatableRead>,
        HybRepo,
        RdbProm,
        _,
    >(
        (
            harn.nucl().repeatable_read(),
            harn.repo(),
            harn.prom(),
            harn.image_pool(),
        ),
        user_token,
        comic_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/comics/{comic_id}/cover/mark-uploaded` — confirm a cover upload.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/cover/mark-uploaded",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    request_body = MarkComicCoverUploadedInstr,
    responses(
        (status = 204, description = "Cover upload confirmed"),
        (status = 403, description = "No perm to modify this comic's cover"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn mark_cover_uploaded(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<MarkComicCoverUploadedInstr>,
) -> HttpNoContent {
    //
    usecase::comic::mark_cover_uploaded::<
        _,
        RdbContext<RepeatableRead>,
        HybRepo,
        _,
    >(
        (
            harn.nucl().repeatable_read(),
            harn.repo(),
            harn.image_pool(),
        ),
        user_token,
        comic_id,
        instr,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/comics/{comic_id}/archive` — archive one comic and clear its active data.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/comics/{comic_id}/archive",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 201, description = "Comic archived", body = HttpBody<ArchiveComicVal>),
        (status = 403, description = "No perm to archive this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn archive(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<ArchiveComicVal> {
    //
    usecase::comic_archive::archive::<
        _,
        RdbContext<Serializable>,
        HybRepo,
        RdbProm,
    >(
        (harn.nucl().serializable(), harn.repo(), harn.prom()),
        user_token,
        comic_id,
    )
    .await?
    .accept(StatusCode::CREATED)
}

/// `DELETE /api/v1/comics/{comic_id}` — delete a comic and descendants.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/comics/{comic_id}",
    tag = "comics",
    params(("comic_id" = String, Path, description = "Comic ID")),
    responses(
        (status = 204, description = "Comic deleted"),
        (status = 403, description = "No perm to delete this comic"),
        (status = 404, description = "Comic not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(comic_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::comic::delete::<_, RdbContext<Serializable>, HybRepo, RdbProm>(
        (harn.nucl().serializable(), harn.repo(), harn.prom()),
        user_token,
        comic_id,
    )
    .await?;

    no_content()
}
