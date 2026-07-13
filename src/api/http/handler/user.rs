//! User handlers: profile read/update, deletion, and avatar upload flow.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use tracing::instrument;

use crate::api::http::handler::util::{
    ensure_current_user, ensure_path_matches_body_id,
};
#[allow(unused_imports)]
use crate::api::http::result::{
    Accept as _, HttpBody, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::user::{
    MarkUserAvatarUploadedParams, ReserveUserAvatarParams,
    ReserveUserAvatarPayload, UpdateUserInfoParams, UserInfoVal,
};
use crate::model::user::UserToken;
use crate::usecase;

/// `GET /api/v1/users/me` — current user's profile.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "users",
    responses(
        (status = 200, description = "Current user profile", body = HttpBody<UserInfoVal>),
        (status = 401, description = "Authentication required"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn get_my_info(
    State(harn): State<AppHarn>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<UserInfoVal> {
    //
    let id = user_token.user_id.clone();

    usecase::user::get_info(
        harn.repo(),
        harn.image_pool(),
        harn.develop(),
        user_token,
        id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `GET /api/v1/users/{user_id}` — a user's profile by id.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID")),
    responses(
        (status = 200, description = "User profile retrieved", body = HttpBody<UserInfoVal>),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "User not found"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn get_info(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(token): Extension<UserToken>,
) -> HttpResult<UserInfoVal> {
    usecase::user::get_info(
        harn.repo(),
        harn.image_pool(),
        harn.develop(),
        token,
        user_id,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `PUT /api/v1/users/{user_id}` — update a user's profile.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    put,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID")),
    request_body = UpdateUserInfoParams,
    responses(
        (status = 204, description = "Profile updated"),
        (status = 422, description = "Path id does not match body id"),
        (status = 403, description = "Cannot modify another user's profile"),
        (status = 409, description = "QID already taken"),
    ),
))]
#[instrument(err, skip(harn, params))]
pub async fn update_info(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<UpdateUserInfoParams>,
) -> HttpNoContent {
    //
    ensure_path_matches_body_id(&user_id, &params.id)?;

    usecase::user::update_info(harn.drive(), harn.repo(), user_token, params)
        .await?;

    no_content()
}

/// `DELETE /api/v1/users/{user_id}` — delete a user account.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    delete,
    path = "/api/v1/users/{user_id}",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 403, description = "Cannot delete another user's account"),
        (status = 404, description = "User not found"),
    ),
))]
#[instrument(err, skip(harn))]
pub async fn delete(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
) -> HttpNoContent {
    //
    usecase::user::delete(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        user_token,
        user_id,
    )
    .await?;

    no_content()
}

/// `POST /api/v1/users/{user_id}/avatar/reserve` — reserve an avatar upload slot.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/users/{user_id}/avatar/reserve",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID (must match authenticated user)")),
    request_body = ReserveUserAvatarParams,
    responses(
        (status = 200, description = "Avatar upload URL reserved", body = HttpBody<ReserveUserAvatarPayload>),
        (status = 403, description = "Cannot modify another user's avatar"),
    ),
))]
#[instrument(err, skip(harn, params))]
pub async fn reserve_avatar(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<ReserveUserAvatarParams>,
) -> HttpResult<ReserveUserAvatarPayload> {
    //
    ensure_current_user(&user_id, &user_token)?;

    usecase::user::reserve_avatar(
        harn.drive(),
        harn.repo(),
        harn.prom(),
        harn.image_pool(),
        user_token,
        params,
    )
    .await?
    .accept(StatusCode::OK)
}

/// `POST /api/v1/users/{user_id}/avatar/mark-uploaded` — confirm an avatar upload.
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    post,
    path = "/api/v1/users/{user_id}/avatar/mark-uploaded",
    tag = "users",
    params(("user_id" = String, Path, description = "Target user ID (must match authenticated user)")),
    request_body = MarkUserAvatarUploadedParams,
    responses(
        (status = 204, description = "Avatar upload confirmed"),
        (status = 403, description = "Cannot confirm another user's avatar"),
    ),
))]
#[instrument(err, skip(harn, params))]
pub async fn mark_avatar_uploaded(
    State(harn): State<AppHarn>,
    Path(user_id): Path<String>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<MarkUserAvatarUploadedParams>,
) -> HttpNoContent {
    //
    usecase::user::mark_avatar_uploaded(
        harn.drive(),
        harn.repo(),
        user_token,
        user_id,
        params,
    )
    .await?;

    no_content()
}
