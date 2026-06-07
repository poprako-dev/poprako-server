use axum::extract::{Request, State};
use axum::http::HeaderName;
use axum::http::header;
use axum::middleware::{self, Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use futures_util::FutureExt as _;
use futures_util::future::BoxFuture;
use tower::Layer;
use tower::ServiceBuilder;
use tower::layer::util::{Identity, Stack};
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::{
    DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
    HttpMakeClassifier, MakeSpan, TraceLayer,
};
use tracing::Instrument;
use uuid::Uuid;

use crate::api::http::auth_token::AUTHORIZATION_BEARER_PREFIX;
use crate::api::http::auth_token::AUTHORIZATION_COOKIE_NAME;
use crate::api::http::result::HttpError;
use crate::domain::compound::user::parse_token;
use crate::domain::result::ExpectedVariant;
use crate::harness::Harness;
use crate::usecase;

use poprako_util::i18n::trl;

type LayerFuture = BoxFuture<'static, Response>;

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

        let Ok(user_token) = parse_token(&harn, &raw_token) else {
            return HttpError::expected(
                &ExpectedVariant::Authentication,
                &trl("error-unauthorized"),
            )
            .into_response();
        };

        // Update last active timestamp on every authenticated request.
        let _ = usecase::user::touch_last_active(&harn, &user_token.user_id).await;

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

const REQUEST_ID_HEADER: &str = "x-request-id";

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

#[derive(Clone, Debug)]
pub struct MakeHttpRequestSpan;

impl<B> MakeSpan<B> for MakeHttpRequestSpan {
    fn make_span(&mut self, request: &axum::http::Request<B>) -> tracing::Span {
        let request_id = request
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_owned())
            .unwrap_or_else(|| Uuid::now_v7().to_string());

        tracing::info_span!("http_request", request_id = %request_id)
    }
}

type HttpTraceLayer = TraceLayer<
    HttpMakeClassifier,
    MakeHttpRequestSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    DefaultOnFailure,
>;

type IdTraceLayerInner = ServiceBuilder<
    Stack<
        PropagateRequestIdLayer,
        Stack<HttpTraceLayer, Stack<SetRequestIdLayer<MakeRequestUuidV7>, Identity>>,
    >,
>;

/// Tower layer that sets, traces, and propagates the request ID.
#[derive(Clone, Debug, Default)]
pub struct IdTraceLayer;

impl IdTraceLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for IdTraceLayer
where
    IdTraceLayerInner: Layer<S>,
{
    type Service = <IdTraceLayerInner as Layer<S>>::Service;

    fn layer(&self, inner: S) -> Self::Service {
        Layer::layer(&id_trace_layer(), inner)
    }
}

fn id_trace_layer() -> IdTraceLayerInner {
    ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static(REQUEST_ID_HEADER),
            MakeRequestUuidV7,
        ))
        .layer(TraceLayer::new_for_http().make_span_with(MakeHttpRequestSpan))
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            REQUEST_ID_HEADER,
        )))
}
