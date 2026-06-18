use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::api::http::result::{Accept as _, HttpError, HttpResult};
use crate::domain::model::aggr::user::UserToken;
use crate::domain::result::ExpectedVariant;
use crate::harness::Harness;
use crate::usecase_legacy;
use crate::usecase_legacy::data_object::member::{
    CreateParams, CreateReply, JoinParams, ListParams, MemberInfo, RoleUpdateParams,
};

fn invalid_role_argument() -> HttpError {
    HttpError::expected(&ExpectedVariant::Argument, &trl("error-member-not-found"))
}

#[utoipa::path(
    post,
    path = "/members",
    tag = "members",
    request_body = CreateParams,
    responses(
        (status = 201, description = "Member created", body = CreateReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Insufficient permissions", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn create(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<CreateParams>,
) -> HttpResult<CreateReply> {
    let reply = usecase_legacy::member::create(&harn, &user_token, params).await?;

    reply.accept(StatusCode::CREATED)
}

#[utoipa::path(
    get,
    path = "/members",
    tag = "members",
    params(ListParams),
    responses(
        (status = 200, description = "Members listed", body = Vec<MemberInfo>),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Insufficient permissions", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list_infos(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<ListParams>,
) -> HttpResult<Vec<MemberInfo>> {
    // Filter out invalid params for listing members, e.g. user_id.
    let list_params = ListParams {
        team_id: params.team_id,
        keyword: params.keyword,
        role: params.role,
        offset: params.offset,
        limit: params.limit,
        includes: params.includes,
        ..Default::default()
    };

    let infos = usecase_legacy::member::list_infos(&harn, &user_token, &list_params).await?;

    infos.accept(StatusCode::OK)
}

#[utoipa::path(
    put,
    path = "/members/{member_id}",
    tag = "members",
    params(
        ("member_id" = String, Path, description = "Member ID")
    ),
    request_body = RoleUpdateParams,
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
    Json(params): Json<RoleUpdateParams>,
) -> HttpResult<()> {
    usecase_legacy::member::update_roles(&harn, &user_token, member_id, params).await?;

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
    usecase_legacy::member::delete(&harn, &user_token, member_id).await?;

    ().accept(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/members/me",
    tag = "members",
    responses(
        (status = 200, description = "Current user memberships listed", body = Vec<MemberInfo>),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
#[instrument(err, skip(harn))]
pub async fn list_my_infos(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Query(params): Query<ListParams>,
) -> HttpResult<Vec<MemberInfo>> {
    // Filter out invalid params for listing current user's memberships.
    let list_params = ListParams {
        user_id: Some(user_token.user_id.clone()),
        offset: params.offset,
        limit: params.limit,
        includes: params.includes,
        ..Default::default()
    };

    let infos = usecase_legacy::member::list_infos(&harn, &user_token, &list_params).await?;

    infos.accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/members/join",
    tag = "members",
    request_body = JoinParams,
    responses(
        (status = 200, description = "Joined team successfully", body = CreateReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 409, description = "Already a team member", body = HttpError)
    )
)]
#[instrument(err, skip(harn, params))]
pub async fn join(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<JoinParams>,
) -> HttpResult<CreateReply> {
    let reply = usecase_legacy::member::join(&harn, &user_token, params).await?;

    reply.accept(StatusCode::OK)
}
