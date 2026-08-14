//! Authentication handlers: register, login, and logout.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use cookie::time::Duration;
use cookie::{Cookie, SameSite};
use tracing::instrument;

use crate::api::http::auth::AUTH_COOKIE_NAME;
#[cfg(feature = "swagger")]
use crate::api::http::result::HttpBody;
use crate::api::http::result::{
    Accept as _, HttpNoContent, HttpResult, no_content,
};
use crate::api::http::state::AppHarn;
use crate::data::instr::auth::{LoginAuthInstr, RegisterAuthInstr};
use crate::data::val::auth::{LoginAuthVal, RegisterAuthVal};
use crate::part::nucl::RepeatableRead;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;

/// `POST /api/v1/auth/register` — registers a user via invitation code.
///
/// Public route. On success, sets the `authorization-token` cookie and returns
/// the new user id with a signed token.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "auth",
    request_body = RegisterAuthInstr,
    responses(
        (status = 201, description = "Registration successful, sets auth cookie", body = HttpBody<RegisterAuthVal>),
        (status = 422, description = "Invalid request parameters"),
        (status = 401, description = "Invalid invitation code"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn register(
    State(harn): State<AppHarn>,
    Json(instr): Json<RegisterAuthInstr>,
) -> HttpResult<RegisterAuthVal> {
    //
    let reply = usecase::auth::register::<
        _,
        RdbContext<RepeatableRead>,
        HybRepo,
        _,
        _,
    >(
        (
            harn.nucl_repeatable_read(),
            harn.repo(),
            harn.auth(),
            harn.develop(),
        ),
        instr,
    )
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
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginAuthInstr,
    responses(
        (status = 200, description = "Login successful, sets auth cookie", body = HttpBody<LoginAuthVal>),
        (status = 401, description = "Invalid credentials"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn login(
    State(harn): State<AppHarn>,
    Json(instr): Json<LoginAuthInstr>,
) -> HttpResult<LoginAuthVal> {
    //
    let reply = usecase::auth::login::<RdbContext<RepeatableRead>, HybRepo, _>(
        (harn.repo(), harn.auth()),
        instr,
    )
    .await?;

    let cookie = auth_cookie(&reply.token);

    reply
        .accept(StatusCode::OK)
        .map(|body| body.with_cookie(&cookie))
}

/// `POST /api/v1/auth/logout` — clears the authorization cookie.
///
/// Public route. Logs the client out by clearing the `authorization-token`
/// cookie. The client will no longer send the token on subsequent requests.
#[cfg_attr(feature = "swagger", utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Logged out successfully, auth cookie cleared"),
    ),
))]
#[instrument(level = "info", skip_all)]
pub async fn logout() -> HttpNoContent {
    //
    let cookie = Cookie::build((AUTH_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(Duration::ZERO)
        .build();

    no_content().map(|body| body.with_cookie(&cookie))
}

// Builds the `authorization-token` HttpOnly cookie used by auth responses.
fn auth_cookie(token: &str) -> Cookie<'static> {
    //
    Cookie::build((AUTH_COOKIE_NAME, format!("Bearer {}", token)))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build()
}
