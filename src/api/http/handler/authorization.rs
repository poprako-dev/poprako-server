use axum::Json;
use axum::extract::State;
use cookie::Cookie;
use cookie::SameSite;

use crate::api::http::middleware::AUTHORIZATION_COOKIE_NAME;
use crate::api::http::result::HttpResponse;
use crate::api::http::result::HttpResult;
use crate::harness::Harness;
use crate::usecase;
use crate::usecase::data_object::user::{SignUpUserParams, SignUpUserReply};

pub async fn sign_up_user(
    State(harn): State<Harness>,
    Json(params): Json<SignUpUserParams>,
) -> HttpResult<SignUpUserReply> {
    let reply = usecase::user::sign_up_user(&harn, params).await?;

    let cookie = Cookie::build((AUTHORIZATION_COOKIE_NAME, format!("Bearer {}", reply.token)))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    Ok(HttpResponse::from(reply).with_cookie(&cookie))
}
