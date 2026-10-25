//! HTTP delivery for versioned annotation editing.

use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{Accept as _, HttpError, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::instr::issue_edit::{
    RenameIssueEditVariantInstr, SavePageIssuesInstr,
};
use crate::data::val::issue_edit::{
    IssueEditPageVal, IssueEditVariantsVal, RenameIssueEditVariantVal,
    SavePageIssuesVal,
};
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::issue_edit as issue_edit_usecase;
use crate::usecase::issue_edit::{
    rename as issue_rename_usecase, save as issue_save_usecase,
};

/// Immutable request identity reused only when retrying the same batch.
#[derive(Deserialize)]
pub struct SaveQuery {
    /// UUID batch identity.
    pub save_id: String,
}

/// GET `/page-artworks/{page_artwork_id}/issues`.
#[cfg_attr(feature = "swagger",
         utoipa::path(
    get,
         path = "/api/v1/page-artworks/{page_artwork_id}/issues",
         tag = "issues",

    params(("page_artwork_id" = String,
         Path,
         description = "Resource identity")),


    responses(
        (status = 200,
         description = "Current review state",
         body = HttpBody<IssueEditPageVal>),

        (status = 403,
         description = "Insufficient review permissions"),

        (status = 409,
         description = "Review, image, or annotation ver changed"),

        (status = 422,
         description = "Invalid target or annotation contents"),

    ),

))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(id): Path<String>,
    Extension(token): Extension<UserToken>,
) -> HttpResult<IssueEditPageVal> {
    //
    Ok(
        issue_edit_usecase::get_info::<_, RdbContext<ReptRead>, RdbRepo, _>(
            (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
            token,
            id,
        )
        .await?
        .accept(StatusCode::OK),
    )
}

/// GET `/chapters/{chapter_id}/issues/variants`.
#[cfg_attr(feature = "swagger",
         utoipa::path(
    get,
         path = "/api/v1/chapters/{chapter_id}/issues/variants",
         tag = "issues",

    params(("chapter_id" = String,
         Path,
         description = "Resource identity")),


    responses(
        (status = 200,
         description = "Current review state",
         body = HttpBody<IssueEditVariantsVal>),

        (status = 403,
         description = "Insufficient review permissions"),

        (status = 409,
         description = "Review, image, or annotation ver changed"),

        (status = 422,
         description = "Invalid target or annotation contents"),

    ),

))]
#[instrument(level = "info", skip_all)]
pub async fn list_variant_infos(
    State(harn): State<AppHarn>,
    Path(id): Path<String>,
    Extension(token): Extension<UserToken>,
) -> HttpResult<IssueEditVariantsVal> {
    //
    Ok(
        issue_edit_usecase::list_variant_infos::<
            _,
            RdbContext<ReptRead>,
            RdbRepo,
        >((harn.nucl().rept_read(), harn.repo()), token, id)
        .await?
        .accept(StatusCode::OK),
    )
}

/// POST `/chapters/{chapter_id}/issues/variants/rename`.
#[cfg_attr(feature = "swagger",
         utoipa::path(
    post,
         path = "/api/v1/chapters/{chapter_id}/issues/variants/rename",
         tag = "issues",

    params(("chapter_id" = String,
         Path,
         description = "Resource identity")),

    request_body = RenameIssueEditVariantInstr,

    responses(
        (status = 200,
         description = "Current review state",
         body = HttpBody<RenameIssueEditVariantVal>),

        (status = 403,
         description = "Insufficient review permissions"),

        (status = 409,
         description = "Review, image, or annotation ver changed"),

        (status = 422,
         description = "Invalid target or annotation contents"),

    ),

))]
#[instrument(level = "info", skip_all)]
pub async fn rename_variant(
    State(harn): State<AppHarn>,
    Path(id): Path<String>,
    Extension(token): Extension<UserToken>,

    Json(instr): Json<RenameIssueEditVariantInstr>,
) -> HttpResult<RenameIssueEditVariantVal> {
    //
    Ok(issue_rename_usecase::rename_variant::<
        _,
        RdbContext<ReptRead>,
        RdbRepo,
        _,
    >(
        (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
        token,
        id,
        instr,
    )
    .await?
    .accept(StatusCode::OK))
}

/// POST `/page-artworks/{page_artwork_id}/issues/save`.
#[cfg_attr(feature = "swagger",
         utoipa::path(
    post,
         path = "/api/v1/page-artworks/{page_artwork_id}/issues/save",
         tag = "issues",

    params(("page_artwork_id" = String,
         Path,
         description = "Resource identity"),
         ("save_id" = String,
         Query,
         description = "Immutable batch UUID")),

    request_body = SavePageIssuesInstr,

    responses(
        (status = 200,
         description = "Current review state",
         body = HttpBody<SavePageIssuesVal>),

        (status = 403,
         description = "Insufficient review permissions"),

        (status = 409,
         description = "Review, image, or annotation ver changed"),

        (status = 422,
         description = "Invalid target or annotation contents"),

    ),

))]
#[instrument(level = "info", skip_all)]
pub async fn save_infos(
    State(harn): State<AppHarn>,
    Path(id): Path<String>,
    Extension(token): Extension<UserToken>,
    Query(query): Query<SaveQuery>,
    Json(instr): Json<SavePageIssuesInstr>,
) -> HttpResult<SavePageIssuesVal> {
    //
    let save_id = Uuid::parse_str(&query.save_id)
        .map_err(|_| HttpError::unprocessable("save_id must be a UUID"))?;

    Ok(
        issue_save_usecase::save_infos::<_, RdbContext<ReptRead>, RdbRepo, _, _>(
            (harn.nucl().rept_read(), harn.repo(), harn.cache(), harn.obj_dept()),
            token,
            id,
            save_id,
            instr,
        )
        .await?
        .accept(StatusCode::OK),
    )
}
