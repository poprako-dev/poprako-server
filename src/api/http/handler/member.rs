use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use poprako_util::page::Page;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpError;
use crate::api::http::result::HttpResult;
use crate::domain::model::value::role::RoleFlag;
use crate::harness::Harness;
use crate::usecase;
use crate::usecase::data_object::member::{
    MemberBase, MemberCreateParams, MemberCreateReply, MemberRoleUpdateParams,
};

#[utoipa::path(
    post,
    path = "/members",
    tag = "members",
    request_body = MemberCreateParams,
    responses(
        (status = 201, description = "Member created", body = MemberCreateReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn create(
    State(harn): State<Harness>,
    Json(params): Json<MemberCreateParams>,
) -> HttpResult<MemberCreateReply> {
    let reply = usecase::member::create(&harn, params).await?;

    reply.accept(StatusCode::CREATED)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberListQuery {
    pub team_id: String,
    pub keyword: Option<String>,
    pub role: Option<u32>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/members",
    tag = "members",
    params(MemberListQuery),
    responses(
        (status = 200, description = "Members listed", body = Vec<MemberBase>),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list(
    State(harn): State<Harness>,
    Query(params): Query<MemberListQuery>,
) -> HttpResult<Vec<MemberBase>> {
    let role = params.role.and_then(RoleFlag::try_from_single_bit);

    let page = Page {
        offset: params.offset.unwrap_or(0) as usize,
        limit: params.limit.unwrap_or(20) as usize,
    };

    let bases = usecase::member::list(
        &harn,
        &params.team_id,
        params.keyword.as_deref(),
        role,
        page,
    )
    .await?;

    bases.accept(StatusCode::OK)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberDetailQuery {
    pub user_id: String,
    pub team_id: String,
}

#[utoipa::path(
    get,
    path = "/members/detail",
    tag = "members",
    params(MemberDetailQuery),
    responses(
        (status = 200, description = "Member detail retrieved", body = MemberBase),
        (status = 400, description = "Member not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn get_detail(
    State(harn): State<Harness>,
    Query(params): Query<MemberDetailQuery>,
) -> HttpResult<MemberBase> {
    let base =
        usecase::member::get_by_user_and_team(&harn, &params.user_id, &params.team_id).await?;

    base.accept(StatusCode::OK)
}

#[utoipa::path(
    put,
    path = "/members/{member_id}",
    tag = "members",
    params(
        ("member_id" = String, Path, description = "Member ID")
    ),
    request_body = MemberRoleUpdateParams,
    responses(
        (status = 200, description = "Member roles updated"),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn update_roles(
    State(harn): State<Harness>,
    Path(member_id): Path<String>,
    Json(params): Json<MemberRoleUpdateParams>,
) -> HttpResult<()> {
    usecase::member::update_roles(&harn, member_id, params).await?;

    ().accept(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/members/{member_id}",
    tag = "members",
    params(
        ("member_id" = String, Path, description = "Member ID")
    ),
    responses(
        (status = 200, description = "Member deleted"),
        (status = 400, description = "Member not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn delete(State(harn): State<Harness>, Path(member_id): Path<String>) -> HttpResult<()> {
    usecase::member::delete(&harn, member_id).await?;

    ().accept(StatusCode::OK)
}
