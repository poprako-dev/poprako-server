use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpError;
use crate::api::http::result::HttpResult;
use crate::domain::model::aggr::user::UserToken;
use crate::domain::model::value::member_inclusion::MemberInclusion;
use crate::domain::model::value::role::RoleFlag;
use crate::domain::result::ExpectedVariant;
use crate::harness::Harness;
use crate::usecase;
use crate::usecase::data_object::member::{
    ListMyMembersParams, MemberBase, MemberCreateParams, MemberCreateReply, MemberJoinParams,
    MemberListParams, MemberMineParams, MemberRoleUpdateParams,
};

fn invalid_role_argument() -> HttpError {
    HttpError::expected(&ExpectedVariant::Argument, &trl("error-member-not-found"))
}

#[utoipa::path(
    post,
    path = "/members",
    tag = "members",
    request_body = MemberCreateParams,
    responses(
        (status = 201, description = "Member created", body = MemberCreateReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Insufficient permissions", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn create(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<MemberCreateParams>,
) -> HttpResult<MemberCreateReply> {
    let reply = usecase::member::create(&harn, &user_token, params).await?;

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
    pub includes: Option<String>,
}

#[utoipa::path(
    get,
    path = "/members",
    tag = "members",
    params(MemberListQuery),
    responses(
        (status = 200, description = "Members listed", body = Vec<MemberBase>),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Insufficient permissions", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<MemberListQuery>,
) -> HttpResult<Vec<MemberBase>> {
    let role = match params.role {
        Some(bits) if bits != 0 => {
            let flag = RoleFlag::try_from_single_bit(bits).ok_or_else(invalid_role_argument)?;
            Some(flag)
        }
        Some(_) => return Err(invalid_role_argument()),
        None => None,
    };

    let page = Page {
        offset: params.offset.unwrap_or(0) as usize,
        limit: params.limit.unwrap_or(20) as usize,
    };

    let usecase_params = MemberListParams {
        team_id: params.team_id,
        keyword: params.keyword,
        role,
        page,
        includes: MemberInclusion::parse(params.includes.as_deref()),
    };

    let bases = usecase::member::list_infos(&harn, &user_token, &usecase_params).await?;

    bases.accept(StatusCode::OK)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListMyMembersQuery {
    pub team_id: String,
    pub includes: Option<String>,
}

#[utoipa::path(
    get,
    path = "/members/detail",
    tag = "members",
    params(ListMyMembersQuery),
    responses(
        (status = 200, description = "Member detail retrieved", body = MemberBase),
        (status = 400, description = "Member not found", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Insufficient permissions", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list_my_members(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<ListMyMembersQuery>,
) -> HttpResult<MemberBase> {
    let usecase_params = ListMyMembersParams {
        team_id: params.team_id,
        includes: MemberInclusion::parse(params.includes.as_deref()),
    };

    let base = usecase::member::list_my_members(&harn, &user_token, &usecase_params).await?;

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
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Insufficient permissions", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn update_roles(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Path(member_id): Path<String>,
    Json(params): Json<MemberRoleUpdateParams>,
) -> HttpResult<()> {
    usecase::member::update_roles(&harn, &user_token, member_id, params).await?;

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
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Insufficient permissions", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Path(member_id): Path<String>,
) -> HttpResult<()> {
    usecase::member::delete(&harn, &user_token, member_id).await?;

    ().accept(StatusCode::OK)
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MemberMineQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub includes: Option<String>,
}

#[utoipa::path(
    get,
    path = "/members/mine",
    tag = "members",
    params(MemberMineQuery),
    responses(
        (status = 200, description = "Current user memberships listed", body = Vec<MemberBase>),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list_mine(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<MemberMineQuery>,
) -> HttpResult<Vec<MemberBase>> {
    let usecase_params = MemberMineParams {
        page: Page {
            offset: params.offset.unwrap_or(0) as usize,
            limit: params.limit.unwrap_or(20) as usize,
        },
        includes: MemberInclusion::parse(params.includes.as_deref()),
    };

    let bases = usecase::member::list_mine(&harn, &user_token, &usecase_params).await?;

    bases.accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/members/join",
    tag = "members",
    request_body = MemberJoinParams,
    responses(
        (status = 200, description = "Joined team successfully", body = MemberCreateReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 409, description = "Already a team member", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn join(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<MemberJoinParams>,
) -> HttpResult<MemberCreateReply> {
    let reply = usecase::member::join(&harn, &user_token, params).await?;

    reply.accept(StatusCode::OK)
}
