//! HTTP middleware: authorization token verification and request latency
//! logging.

/// Authorization token middleware.
pub mod auth;
// Custom request latency logging is disabled in favor of `TraceLayer`.
// pub mod latency;
/// Sliding-window HTTP response metrics.
pub mod metric;
/// Rate limiting middleware.
pub mod rate_limit;
/// Request ID and tracing middleware.
pub mod trace;
