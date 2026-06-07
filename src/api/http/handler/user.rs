use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;

use crate::api::http::handler::util::ensure_current_user;
use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpError;
use crate::api::http::result::HttpResult;
use crate::domain::model::aggr::user::UserToken;
use crate::harness::Harness;
use crate::usecase;
use crate::usecase::data_object::user::{
    ReserveAvatarParams, ReserveAvatarReply, UserBase, UserInfoUpdateParams,
};

#[utoipa::path(
    get,
    path = "/users/{user_id}",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "Target user ID")
    ),
    responses(
        (status = 200, description = "User info retrieved", body = UserBase),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 404, description = "User not found", body = HttpError)
    )
)]
pub async fn get_info(
    State(harn): State<Harness>,
    Path(user_id): Path<String>,
) -> HttpResult<UserBase> {
    let base = usecase::user::get_info(&harn, &user_id).await?;

    base.accept(StatusCode::OK)
}

#[utoipa::path(
    get,
    path = "/users/me",
    tag = "users",
    responses(
        (status = 200, description = "Current user info retrieved", body = UserBase),
        (status = 401, description = "Authentication required", body = HttpError)
    )
)]
pub async fn get_my_info(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<UserBase> {
    let base = usecase::user::get_info(&harn, &user_token.user_id).await?;

    base.accept(StatusCode::OK)
}

#[utoipa::path(
    put,
    path = "/users/me",
    tag = "users",
    request_body = UserInfoUpdateParams,
    responses(
        (status = 200, description = "Profile updated"),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 409, description = "QID already taken", body = HttpError)
    )
)]
pub async fn update_info(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<UserInfoUpdateParams>,
) -> HttpResult<()> {
    usecase::user::update_info(&harn, user_token, params).await?;

    ().accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/users/{user_id}/avatar/reserve",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "Target user ID (must match authenticated user)")
    ),
    request_body = ReserveAvatarParams,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = ReserveAvatarReply),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Cannot modify another user's avatar", body = HttpError)
    )
)]
pub async fn reserve_avatar(
    State(harn): State<Harness>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<ReserveAvatarParams>,
) -> HttpResult<ReserveAvatarReply> {
    ensure_current_user(&user_id, &user_token)?;

    let reply = usecase::user::reserve_avatar(&harn, user_token, params).await?;

    reply.accept(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/users/{user_id}/avatar/mark-uploaded",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "Target user ID (must match authenticated user)")
    ),
    responses(
        (status = 200, description = "Avatar upload confirmed"),
        (status = 400, description = "Invalid request parameters", body = HttpError),
        (status = 401, description = "Authentication required", body = HttpError),
        (status = 403, description = "Cannot confirm another user's avatar", body = HttpError)
    )
)]
pub async fn mark_avatar_uploaded(
    State(harn): State<Harness>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<()> {
    ensure_current_user(&user_id, &user_token)?;

    usecase::user::mark_avatar_uploaded(&harn, user_token).await?;

    ().accept(StatusCode::OK)
}
