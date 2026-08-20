//! Active HTTP API module: result types, auth token constants, middleware,
//! handlers, router, OpenAPI, and server entry point.

mod shared;

// FIXME: why all pub?

/// Authentication utilities for the HTTP API.
pub mod auth;
/// HTTP request handlers grouped by resource.
pub mod handler;
/// HTTP middleware: authorization, latency, rate limiting.
pub mod middleware;

/// OpenAPI documentation types.
#[cfg(feature = "swagger")]
pub mod openapi;

/// HTTP result types and response utilities.
pub mod result;
/// HTTP router definition.
pub mod router;
/// HTTP server entry point.
pub mod server;
/// HTTP application state.
pub mod state;
