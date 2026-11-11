//! Active HTTP API module: result types, auth token constants, middleware,
//! handlers, router, OpenAPI, and server entry point.
// Shared HTTP metrics and transport support.
mod shared;
// Authentication utilities for the HTTP API.
mod auth;
// HTTP request handlers grouped by resource.
mod handler;
// HTTP result types and response utilities.
mod result;
// HTTP router definition.
mod router;
// HTTP application state.
mod state;

/// HTTP middleware: authorization, latency, rate limiting.
pub mod middleware;
/// HTTP wire objects for request bodies and response payloads.
pub mod object;
/// HTTP server entry point.
pub mod server;

/// OpenAPI documentation types.
#[cfg(feature = "swagger")]
pub mod openapi;
