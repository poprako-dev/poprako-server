use axum::Router;
use axum::extract::{Request, State};
use axum::http::HeaderName;
use axum::http::header;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use tower::ServiceBuilder;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::TraceLayer;
use tracing::{Level, info_span, instrument};
use uuid::Uuid;

use crate::api::harness::Harness;
use crate::api::http::handler::result::HttpError;
use crate::domain::external::token::TokenParse;
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
async fn authorize(State(harn): State<Harness>, mut request: Request, next: Next) -> Response {
    let raw_token = extract_token(&request);

    let Ok(user_token) = harn.parse(&raw_token) else {
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

/// Wraps a [`Router<Harness>`] with authorization middleware.
///
/// Applies [`authorize`] via [`middleware::from_fn_with_state`] so that
/// every request flowing through the returned router must carry a valid
/// authorization token (cookie or `Authorization` header).
/// Requests that fail validation receive a 401 response.
pub fn with_authorization(router: Router<Harness>, harn: Harness) -> Router<Harness> {
    router.layer(middleware::from_fn_with_state(harn, authorize))
}

/// Name of the header that carries the request ID.
///
/// Used by [`with_request_id`] for both incoming propagation
/// (reading an existing ID forwarded by a proxy) and outgoing
/// propagation (writing the ID to the response headers).
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// A [`MakeRequestId`] implementation that generates UUID v7.
///
/// Unlike [`tower_http::request_id::MakeRequestUuid`] (which uses
/// UUID v4), this produces time-sortable UUID v7 so that request
/// IDs are naturally ordered in logs and databases.
#[derive(Clone, Debug)]
pub struct MakeRequestUuidV7;

impl MakeRequestId for MakeRequestUuidV7 {
    fn make_request_id<B>(&mut self, _request: &axum::http::Request<B>) -> Option<RequestId> {
        let uuid = Uuid::now_v7().to_string();
        axum::http::HeaderValue::from_str(&uuid)
            .ok()
            .map(RequestId::new)
    }
}

/// Wraps a [`Router<Harness>`] with request-id and tracing middleware.
///
/// The middleware stack, composed via [`ServiceBuilder`], provides:
///
/// 1. **Request ID generation** — if the incoming request lacks an
///    `x-request-id` header, a UUID v7 is generated and attached.
/// 2. **Structured tracing** — every request is instrumented with an
///    [`info_span`] that carries the request-id, method, and URI.
/// 3. **Response propagation** — the request-id is copied from the
///    request to the response headers so clients can correlate.
pub fn with_request_id(router: Router<Harness>) -> Router<Harness> {
    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);

    router.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(
                x_request_id.clone(),
                MakeRequestUuidV7,
            ))
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                    let request_id = request
                        .headers()
                        .get(REQUEST_ID_HEADER)
                        .and_then(|v| v.to_str().ok())
                        .map(|v| v.to_owned())
                        .unwrap_or_else(|| Uuid::now_v7().to_string());

                    info_span!("http_request", request_id = request_id)
                }),
            )
            .layer(PropagateRequestIdLayer::new(x_request_id)),
    )
}
