//! Active HTTP API module: result types, auth token constants, middleware,
//! handlers, router, OpenAPI, and server entry point.

pub use shared::prometheus::init_prometheus;

mod shared;

// FIXME: why all pub?

/// Authentication utilities for the HTTP API.
pub mod auth;

/// HTTP request handlers grouped by resource.
pub mod handler;
/// HTTP middleware: authorization, latency, rate limiting.
pub mod middleware;
/// HTTP router definition.
pub mod router;
/// HTTP server entry point.
pub mod server;

/// OpenAPI documentation types.
#[cfg(feature = "swagger-ui")]
pub mod openapi;

/// HTTP result types and response utilities.
pub mod result;
/// HTTP application state.
pub mod state;
