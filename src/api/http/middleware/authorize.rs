use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::{self, Next, from_fn_with_state};
use axum::response::IntoResponse;
use futures_util::FutureExt as _;
use tower::Layer;
use tracing::Instrument as _;

use poprako_util::i18n::trl;

use crate::api::http::auth_token::{AUTHORIZATION_BEARER_PREFIX, AUTHORIZATION_COOKIE_NAME};
use crate::api::http::middleware::LayerFuture;
use crate::api::http::result::HttpError;
use crate::domain::complex::user::UserComplex;
use crate::domain::result::ExpectedVariant;
use crate::harness::Harness;
use crate::usecase_legacy;

type LayerFn = fn(State<Harness>, Request, Next) -> LayerFuture;
type FromFnLayer = middleware::FromFnLayer<LayerFn, Harness, (State<Harness>, Request)>;

/// Tower layer that validates the authorization token.
///
/// Reads the token from the `authorization-token` cookie first, falling back to
/// the `Authorization` header. The parsed token is inserted into request
/// extensions for handlers that need the current user.
#[derive(Clone)]
pub struct AuthorizeLayer {
    layer: FromFnLayer,
}

impl AuthorizeLayer {
    pub fn new(harn: Harness) -> Self {
        Self {
            layer: from_fn_with_state(harn, authorize as LayerFn),
        }
    }
}

impl<S> Layer<S> for AuthorizeLayer
where
    FromFnLayer: Layer<S>,
{
    type Service = <FromFnLayer as Layer<S>>::Service;

    fn layer(&self, inner: S) -> Self::Service {
        self.layer.layer(inner)
    }
}

fn authorize(State(harn): State<Harness>, mut request: Request, next: Next) -> LayerFuture {
    async move {
        let raw_token = extract_token(&request);

        let Ok(user_token) = UserComplex::parse_token(&harn, &raw_token) else {
            return HttpError::expected(
                &ExpectedVariant::Authentication,
                &trl("error-unauthorized"),
            )
            .into_response();
        };

        // Update last active timestamp on every authenticated request.
        let _ = usecase_legacy::user::touch_last_active(&harn, &user_token.user_id).await;

        request.extensions_mut().insert(user_token);

        next.run(request).await
    }
    .instrument(tracing::debug_span!("authorize"))
    .boxed()
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
