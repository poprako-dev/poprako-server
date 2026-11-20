//! Terminology-entry handlers.
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::ensure_path_matches_body_id;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::object::term::{
    CreateTermBody, CreateTermPayload, ListTermInfosBody, TermInfoObj,
    TermListQuery, UpdateTermInfoBody,
};
use crate::api::http::result::{
    AcceptInto as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::RdbRepo;
use crate::shared::RdbContext;
use crate::usecase::term as term_usecase;

/// `POST /api/v1/terms` — create a terminology entry.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/terms",
    tag = "terms",
    request_body = CreateTermBody,
    responses(
        (status = 201, description = "Term \
        created", body = HttpBody<CreateTermPayload>),
        (status = 403, description = "Team translator or proofreader role \
        required"),
        (status = 422, description = "Invalid term, duplicate source, or \
        missing termbase"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn create(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<CreateTermBody>,
) -> HttpResult<CreateTermPayload> {
    //
    Ok(term_usecase::create::<_, RdbContext<ReptRead>, RdbRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::CREATED))
}

/// `GET /api/v1/termbases/{termbase_id}/terms` — list terms in one base.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/termbases/{termbase_id}/terms",
    tag = "terms",
    params(("termbase_id" = String, Path, description = "Termbase ID"),
        TermListQuery),
    responses(
        (status = 200, description = "Terms \
        listed", body = HttpBody<Vec<TermInfoObj>>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Termbase not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(termbase_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Query(query): Query<TermListQuery>,
) -> HttpResult<Vec<TermInfoObj>> {
    //
    let instr = ListTermInfosBody {
        termbase_id,
        fuzzy_source: query.fuzzy_source,
        offset: query.offset,
        limit: query.limit,
    };

    Ok(term_usecase::list_infos::<RdbContext<ReptRead>, RdbRepo>(
        (harn.repo(),),
        user_token,
        instr.into(),
    )
    .await?
    .accept_into(StatusCode::OK))
}

/// `GET /api/v1/terms/{term_id}` — fetch a terminology entry.
#[cfg_attr(feature = "swagger", utoipa::path(
    get,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    responses(
        (status = 200, description = "Term \
        retrieved", body = HttpBody<TermInfoObj>),
        (status = 403, description = "Team membership required"),
        (status = 422, description = "Term not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<TermInfoObj> {
    //
    Ok(term_usecase::get_info::<RdbContext<ReptRead>, RdbRepo>(
        (harn.repo(),),
        user_token,
        term_id,
    )
    .await?
    .accept_into(StatusCode::OK))
}

/// `PUT /api/v1/terms/{term_id}` — replace terminology-entry fields.
#[cfg_attr(feature = "swagger", utoipa::path(
    put,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    request_body = UpdateTermInfoBody,
    responses(
        (status = 204, description = "Term updated"),
        (status = 403, description = "Team translator or proofreader role \
        required"),
        (status = 422, description = "Invalid term, duplicate source, or \
        path mismatch"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(instr): Json<UpdateTermInfoBody>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&term_id, &instr.id)?;

    term_usecase::update_info::<_, RdbContext<ReptRead>, RdbRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        instr.into(),
    )
    .await?;

    Ok(no_content())
}

/// `DELETE /api/v1/terms/{term_id}` — delete a terminology entry.
#[cfg_attr(feature = "swagger", utoipa::path(
    delete,
    path = "/api/v1/terms/{term_id}",
    tag = "terms",
    params(("term_id" = String, Path, description = "Term ID")),
    responses(
        (status = 204, description = "Term deleted"),
        (status = 403, description = "Team translator or proofreader role \
        required"),
        (status = 422, description = "Term not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(term_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    term_usecase::delete::<_, RdbContext<ReptRead>, RdbRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        user_token,
        term_id,
    )
    .await?;

    Ok(no_content())
}
