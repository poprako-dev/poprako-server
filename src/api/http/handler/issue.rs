//! Whole-chapter review import and read-only Chapter queries.

use axum::extract::{Extension, Json, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;

use crate::api::http::result::{Accept as _, HttpResult};
use crate::api::http::state::AppHarn;
use crate::data::instr::issue::ImportChapterIssuesInstr;
use crate::data::val::issue::ImportChapterIssuesVal;
use crate::data::view::issue::IssueInfoView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase::issue as issue_usecase;

/// Replaces the Chapter's single current review without advancing workflow.
#[cfg_attr(feature = "swagger", utoipa::path(
    post, path = "/api/v1/chapters/{chapter_id}/issues/import", tag = "issues",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    request_body = ImportChapterIssuesInstr,
    responses(
        (status = 200, description = "Current review replaced completely", body = HttpBody<ImportChapterIssuesVal>),
        (status = 403, description = "Current REVIEWER assignment required; administrator status alone does not authorize import"),
        (status = 422, description = "Missing Chapter, published Chapter, or invalid issue input"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn import_issue(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(token): Extension<UserToken>,
    Json(instr): Json<ImportChapterIssuesInstr>,
) -> HttpResult<ImportChapterIssuesVal> {
    //
    issue_usecase::import_issue::<_, RdbContext<ReptRead>, HybRepo>(
        (harn.nucl().rept_read(), harn.repo()),
        token,
        chapter_id,
        instr,
    )
    .await?
    .accept(StatusCode::OK)
}

/// Reads the Chapter's issues ordered by composite page and issue position; an empty review returns [].
#[cfg_attr(feature = "swagger", utoipa::path(
    get, path = "/api/v1/chapters/{chapter_id}/issues", tag = "issues",
    params(("chapter_id" = String, Path, description = "Chapter ID")),
    responses(
        (status = 200, description = "Ordered current Chapter issues", body = HttpBody<Vec<IssueInfoView>>),
        (status = 403, description = "Team membership or chapter assignment required"),
        (status = 422, description = "Chapter not found"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    State(harn): State<AppHarn>,
    Path(chapter_id): Path<String>,
    Extension(token): Extension<UserToken>,
) -> HttpResult<Vec<IssueInfoView>> {
    //
    issue_usecase::list_infos::<RdbContext<ReptRead>, HybRepo>(
        (harn.repo(),),
        token,
        chapter_id,
    )
    .await?
    .accept(StatusCode::OK)
}
