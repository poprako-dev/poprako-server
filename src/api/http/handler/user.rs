use axum::Json;
use axum::extract::{Extension, State};
use cookie::Cookie;
use cookie::SameSite;

use crate::api::http::auth_token::AUTHORIZATION_COOKIE_NAME;
use crate::api::http::result::HttpResponse;
use crate::api::http::result::HttpResult;
use crate::domain::model::aggr::user::UserToken;
use crate::harness::Harness;
use crate::usecase;
use crate::usecase::data_object::user::{
    ReserveAvatarParams, ReserveAvatarReply, SignInParams, SignInReply, UserBase,
    UserInfoUpdateParams,
};

pub async fn sign_in(
    State(harn): State<Harness>,
    Json(params): Json<SignInParams>,
) -> HttpResult<SignInReply> {
    let reply = usecase::user::sign_in(&harn, params).await?;

    let cookie = Cookie::build((AUTHORIZATION_COOKIE_NAME, format!("Bearer {}", reply.token)))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    Ok(HttpResponse::from(reply).with_cookie(&cookie))
}

pub async fn get_info(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<UserBase> {
    let base = usecase::user::get_info(&harn, &user_token.user_id).await?;

    Ok(HttpResponse::from(base))
}

pub async fn update_info(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<UserInfoUpdateParams>,
) -> HttpResult<()> {
    usecase::user::update_info(&harn, user_token, params).await?;

    Ok(HttpResponse::from(()))
}

pub async fn reserve_avatar(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
    Json(params): Json<ReserveAvatarParams>,
) -> HttpResult<ReserveAvatarReply> {
    let reply = usecase::user::reserve_avatar(&harn, user_token, params).await?;

    Ok(HttpResponse::from(reply))
}

pub async fn mark_avatar_uploaded(
    State(harn): State<Harness>,
    Extension(user_token): Extension<UserToken>,
) -> HttpResult<()> {
    usecase::user::mark_avatar_uploaded(&harn, user_token).await?;

    Ok(HttpResponse::from(()))
}
