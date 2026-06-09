use axum::response::Response;
use futures_util::future::BoxFuture;

pub mod authorize;
pub mod latency;
pub mod trace;

type LayerFuture = BoxFuture<'static, Response>;
