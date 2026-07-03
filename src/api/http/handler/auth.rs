//! Authentication handlers: register and login.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use cookie::{Cookie, SameSite};

use tracing::instrument;

use crate::api::http::auth_token::AUTH_COOKIE_NAME;
use crate::api::http::result::Accept as _;
use crate::api::http::result::HttpResult;
use crate::api::http::state::AppHarn;
use crate::data::auth::{LoginData, LoginVal, RegisterData, RegisterVal};
use crate::usecase;

/// Builds the `authorization-token` HttpOnly cookie carrying the bearer token.
fn auth_cookie(token: &str) -> Cookie<'static> {
    Cookie::build((AUTH_COOKIE_NAME, format!("Bearer {}", token)))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build()
}

/// `POST /api/v1/auth/register` — registers a user via invitation code.
///
/// Public route. On success, sets the `authorization-token` cookie and returns
/// the new user id with a signed token.
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "auth",
    request_body = RegisterData,
    responses(
        (status = 201, description = "Registration successful, sets auth cookie", body = RegisterVal),
        (status = 400, description = "Invalid request parameters"),
        (status = 401, description = "Invalid invitation code"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn register(
    State(harn): State<AppHarn>,
    Json(data): Json<RegisterData>,
) -> HttpResult<RegisterVal> {
    let reply =
        usecase::auth::register(harn.drive(), harn.repo(), harn.auth(), harn.develop(), data)
            .await?;

    let cookie = auth_cookie(&reply.token);

    reply
        .accept(StatusCode::CREATED)
        .map(|body| body.with_cookie(&cookie))
}

/// `POST /api/v1/auth/login` — authenticates a user by QQ id and password.
///
/// Public route. On success, sets the `authorization-token` cookie and returns
/// the user id with a signed token.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginData,
    responses(
        (status = 200, description = "Login successful, sets auth cookie", body = LoginVal),
        (status = 401, description = "Invalid credentials"),
    ),
)]
#[instrument(err, skip(harn, data))]
pub async fn login(
    State(harn): State<AppHarn>,
    Json(data): Json<LoginData>,
) -> HttpResult<LoginVal> {
    let reply = usecase::auth::login(harn.repo(), harn.auth(), data).await?;

    let cookie = auth_cookie(&reply.token);

    reply
        .accept(StatusCode::OK)
        .map(|body| body.with_cookie(&cookie))
}
