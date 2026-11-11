//! HTTP delivery for versioned annotation editing.
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::StatusCode;
use tracing::instrument;
use uuid::Uuid;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::issue_edit::{
    IssueEditPagePayload, IssueEditVariantsPayload, RenameIssueEditVariantBody,
    RenameIssueEditVariantPayload, SavePageIssuesBody, SavePageIssuesPayload,
    SaveQuery,
};
use crate::api::http::result::{AcceptInto as _, HttpError, HttpResult};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::issue_edit as issue_edit_usecase;
use crate::usecase::issue_edit::{
    rename as issue_rename_usecase, save as issue_save_usecase,
};

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
         body = HttpBody<IssueEditPagePayload>),

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
) -> HttpResult<IssueEditPagePayload> {
    //
    Ok(
        issue_edit_usecase::get_info::<_, RdbContext<ReptRead>, RdbRepo, _>(
            (harn.nucl().rept_read(), harn.repo(), harn.obj_dept()),
            token,
            id,
        )
        .await?
        .accept_into(StatusCode::OK),
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
         body = HttpBody<IssueEditVariantsPayload>),

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
) -> HttpResult<IssueEditVariantsPayload> {
    //
    Ok(
        issue_edit_usecase::list_variant_infos::<
            _,
            RdbContext<ReptRead>,
            RdbRepo,
        >((harn.nucl().rept_read(), harn.repo()), token, id)
        .await?
        .accept_into(StatusCode::OK),
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

    request_body = RenameIssueEditVariantPayload,

    responses(
        (status = 200,
         description = "Current review state",
         body = HttpBody<RenameIssueEditVariantPayload>),

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

    Json(instr): Json<RenameIssueEditVariantBody>,
) -> HttpResult<RenameIssueEditVariantPayload> {
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
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::OK))
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

    request_body = SavePageIssuesPayload,

    responses(
        (status = 200,
         description = "Current review state",
         body = HttpBody<SavePageIssuesPayload>),

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
    Json(instr): Json<SavePageIssuesBody>,
) -> HttpResult<SavePageIssuesPayload> {
    //
    let save_id = Uuid::parse_str(&query.save_id)
        .map_err(|_| HttpError::unprocessable("save_id must be a UUID"))?;

    Ok(
        issue_save_usecase::save_infos::<_, RdbContext<ReptRead>, RdbRepo, _, _>(
            (harn.nucl().rept_read(), harn.repo(), harn.cache(), harn.obj_dept()),
            token,
            id,
            save_id,
            instr.into(),
        )
        .await?.accept_into(StatusCode::OK),
    )
}
