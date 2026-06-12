use std::time::Instant;

use axum::extract::Request;
use axum::middleware::{self, from_fn, Next};
use futures_util::FutureExt as _;
use tower::Layer;

use crate::api::http::middleware::LayerFuture;

type LogLatencyFn = fn(Request, Next) -> LayerFuture;
type LogLatencyFromFnLayer = middleware::FromFnLayer<LogLatencyFn, (), (Request,)>;

/// Tower layer that logs the latency of each HTTP request.
///
/// Records the time before and after the request is processed by the
/// inner service, and emits a `tracing::info!` event with the duration.
#[derive(Clone)]
pub struct LogLatencyLayer {
    layer: LogLatencyFromFnLayer,
}

impl LogLatencyLayer {
    pub fn new() -> Self {
        Self {
            layer: from_fn(log_latency as LogLatencyFn),
        }
    }
}

impl<S> Layer<S> for LogLatencyLayer
where
    LogLatencyFromFnLayer: Layer<S>,
{
    type Service = <LogLatencyFromFnLayer as Layer<S>>::Service;

    fn layer(&self, inner: S) -> Self::Service {
        self.layer.layer(inner)
    }
}

fn log_latency(request: Request, next: Next) -> LayerFuture {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    async move {
        let response = next.run(request).await;
        let duration = start.elapsed();

        tracing::info!(
            method = %method,
            uri = %uri,
            latency_ms = duration.as_secs_f64() * 1000.0,
            "request latency",
        );

        response
    }
    .boxed()
}
