//! HTTP middleware: authorization token verification and request latency
//! logging.

// Custom request latency logging is disabled in favor of `TraceLayer`.
// pub mod latency;
/// Sliding-window HTTP response metrics.
pub use metric::record_response_metric;

/// Authorization token middleware.
pub mod auth;
mod metric;
/// Rate limiting middleware.
pub mod rate_limit;
/// Request ID and tracing middleware.
pub mod trace;
