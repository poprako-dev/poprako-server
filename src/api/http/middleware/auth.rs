//! Authorization middleware.
//!
//! Reads the bearer token from the `authorization-token` cookie first, then
//! falls back to the `Authorization: Bearer <token>` header. On success the
//! decoded [`UserToken`] is inserted into request extensions for handlers.

use axum::extract::Request;
use axum::extract::State;
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::api::http::auth_token::{AUTH_BEARER_PREFIX, AUTH_COOKIE_NAME};
use crate::api::http::result::HttpError;
use crate::api::http::state::AppHarn;
use crate::part::auth::TokenAuth as _;
use crate::usecase;

/// `from_fn` authorization handler applied to protected routes.
pub async fn authorize(State(harn): State<AppHarn>, mut request: Request, next: Next) -> Response {
    let raw_token = extract_token(&request);

    let user_token = match harn.auth().verify_token(&raw_token) {
        Ok(token) => token,
        Err(err) => return HttpError::from(err).into_response(),
    };

    if let Err(e) =
        usecase::user::touch_last_active(harn.drive(), harn.repo(), user_token.clone()).await
    {
        tracing::warn!(error = ?e, "failed to touch last_active");
    }

    request.extensions_mut().insert(user_token);

    next.run(request).await
}

/// Extracts the raw token string from the request.
///
/// Prefers the `authorization-token` cookie; falls back to the `Authorization`
/// header, stripping the `Bearer ` prefix when present.
pub fn extract_token(request: &Request) -> String {
    if let Some(cookie) = request.headers().get(header::COOKIE)
        && let Ok(cookie_str) = cookie.to_str()
    {
        for part in cookie_str.split(';') {
            if let Some((name, value)) = part.trim().split_once('=')
                && name.trim() == AUTH_COOKIE_NAME
            {
                return value.trim().to_string();
            }
        }
    }

    if let Some(auth) = request.headers().get(header::AUTHORIZATION)
        && let Ok(auth_str) = auth.to_str()
    {
        return auth_str
            .strip_prefix(AUTH_BEARER_PREFIX)
            .unwrap_or(auth_str)
            .to_string();
    }

    String::new()
}
