use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tracing::{Level, instrument};

use crate::api::harness::Harness;
use crate::api::http::handler::result::HttpError;
use crate::domain::external::token::TokenCodec;
use crate::domain::model::aggregate::user::UserToken;
use crate::domain::result::ExpectedVariant;
use crate::util::i18n::trl;

/// Name of the cookie that carries the authorization token.
pub const AUTHORIZATION_COOKIE_NAME: &str = "authorization-token";

/// Prefix to strip from the `Authorization` header value.
pub const AUTHORIZATION_BEARER_PREFIX: &str = "Bearer ";

/// Extension key for the parsed [`UserToken`].
///
/// Inserted by [`authorize`] into [`Request::extensions`].
/// Handlers consume it via the request extensions API.
#[derive(Clone, Debug)]
pub struct AuthUser(pub UserToken);

/// Axum middleware that validates the authorization token.
///
/// Reads the token from the `authorization-token` cookie first,
/// falling back to the `Authorization` header. Parses the token
/// via [`TokenCodec`] and stores the resulting [`UserToken`] as
/// an [`AuthUser`] extension. Returns 401 on any failure.
#[instrument(skip(request, harn), level = Level::DEBUG)]
pub async fn authorize(State(harn): State<Harness>, mut request: Request, next: Next) -> Response {
    let raw = extract_token(&request);

    let Ok(user_token) = harn.parse(&raw) else {
        return HttpError::expected(&ExpectedVariant::Authentication, &trl("error-unauthorized"))
            .into_response();
    };

    request.extensions_mut().insert(AuthUser(user_token));

    next.run(request).await
}

/// Extracts the raw token string from the request.
///
/// Prefers the `authorization-token` cookie; falls back to
/// the `Authorization` header (stripping the `"Bearer "` prefix).
fn extract_token(request: &Request) -> String {
    // 1. Try the cookie first.
    if let Some(cookie) = request.headers().get(header::COOKIE)
        && let Ok(cookie_str) = cookie.to_str()
    {
        for part in cookie_str.split(';') {
            if let Some((name, value)) = part.trim().split_once('=')
                && name.trim() == AUTHORIZATION_COOKIE_NAME
            {
                return value.trim().to_string();
            }
        }
    }

    // 2. Fallback to the Authorization header.
    if let Some(auth) = request.headers().get(header::AUTHORIZATION)
        && let Ok(auth_str) = auth.to_str()
    {
        return auth_str
            .strip_prefix(AUTHORIZATION_BEARER_PREFIX)
            .unwrap_or(auth_str)
            .to_string();
    }

    String::new()
}
