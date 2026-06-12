use axum::http::HeaderName;
use tower::layer::util::{Identity, Stack};
use tower::{Layer, ServiceBuilder};
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::{
    DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
    HttpMakeClassifier, MakeSpan, TraceLayer,
};
use uuid::Uuid;

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
